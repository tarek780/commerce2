use std::sync::Arc;

use axum::Router;
use sqlx::{PgPool, postgres::PgPoolOptions};
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::{postgres::Postgres, redis::Redis};

use auth_service::state::AppState;

pub const REGISTER: &str = "/api/v1/auth/register";
pub const LOGIN: &str = "/api/v1/auth/login";
pub const REFRESH: &str = "/api/v1/auth/refresh";
pub const LOGOUT: &str = "/api/v1/auth/logout";
pub const VERIFY_EMAIL: &str = "/api/v1/auth/verify-email";
pub const DELETE_USER: &str = "/api/v1/auth/delete";
pub const ME: &str = "/api/v1/users/me";

pub struct TestContext {
    pub _pg: ContainerAsync<Postgres>,
    pub _redis: ContainerAsync<Redis>,
    pub pool: PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub state: Arc<AppState>,
}

impl TestContext {
    pub async fn new() -> Self {
        unsafe {
            std::env::set_var("SECRET_KEY", "test-secret-key-32-chars-minimum-xx");
            std::env::set_var("RUST_LOG", "info");
        }

        let pg = Postgres::default()
            .start()
            .await
            .expect("start postgres container");
        let pg_port = pg.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("connect test postgres");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        let redis_c = Redis::default().start().await.expect("start redis");
        let redis_port = redis_c.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://127.0.0.1:{redis_port}");
        let redis_client = redis::Client::open(redis_url).expect("redis client");
        let redis = redis::aio::ConnectionManager::new(redis_client)
            .await
            .expect("redis conn manager");

        let state = Arc::new(AppState::new_for_tests(pool.clone(), redis.clone()));

        Self {
            _pg: pg,
            _redis: redis_c,
            pool,
            redis,
            state,
        }
    }

    pub fn router(&self) -> Router {
        auth_service::routes::build_router(self.state.clone())
    }
}

pub async fn truncate_all(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE refresh_tokens, users RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .ok();
}

pub async fn issue_email_token_for(redis: &redis::aio::ConnectionManager, email: &str) -> String {
    auth_service::helpers::email_store::issue_token(redis, email, 600, 60)
        .await
        .expect("issue email token")
}

// ====== مساعدات HTTP ======

use axum::body::Body;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use serde_json::Value;

pub fn json_req(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub fn get_req(uri: &str, bearer: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap()
}

pub fn patch_req(uri: &str, bearer: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("PATCH")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

pub async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

pub async fn register_user(app: &axum::Router, email: &str) -> Value {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            REGISTER,
            serde_json::json!({
                "full_name": "Test User",
                "username": "tester",
                "u_role": "user",
                "email": email,
                "password": "supersecret1",
                "display_name": "Initial"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    body_json(resp).await
}
