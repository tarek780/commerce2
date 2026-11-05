#[allow(dead_code)]
use std::sync::Arc;

use auth_service::state::AppState;
use axum::Router;

use axum::body::Body;
use axum::http::{Request, header};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use testcontainers::GenericImage;
use testcontainers::{
    ContainerAsync,
    core::{ImageExt, WaitFor},
    runners::AsyncRunner,
};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;

pub const REGISTER: &str = "/api/v1/auth/register";
pub const LOGIN: &str = "/api/v1/auth/login";
pub const REFRESH: &str = "/api/v1/auth/refresh";
pub const LOGOUT: &str = "/api/v1/auth/logout";
pub const VERIFY_EMAIL: &str = "/api/v1/auth/verify-email";
pub const DELETE_USER: &str = "/api/v1/auth/delete";
pub const ME: &str = "/api/v1/users/me";

pub struct TestContext {
    pub _pg: ContainerAsync<GenericImage>,
    pub _redis: ContainerAsync<GenericImage>,
    pub pool: PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub state: Arc<AppState>,
}

impl TestContext {
    // pub async fn new() -> Self {
    //     dotenvy::dotenv().ok();
    //     let pg_image = GenericImage::new("postgres", "16.15-alpine3.24")
    //         .with_wait_for(WaitFor::message_on_stdout(
    //             "database system is ready to accept connections",
    //         ))
    //         .with_env_var("POSTGRES_USER", "myuser")
    //         .with_env_var("POSTGRES_PASSWORD", "mypassword")
    //         .with_env_var("POSTGRES_DB", "mydb");
    //     let pg_container = pg_image
    //         .start()
    //         .await
    //         .expect("Failed to start Postgres container");
    //     tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    //     let pg_port = match pg_container.get_host_port_ipv4(5432).await {
    //         Ok(c) => c,
    //         Err(e) => {
    //             let _ = stdout()
    //                 .write_all(format!("Failed to get Postgres container port: {}\n", e).as_bytes())
    //                 .await;
    //             panic!("Failed to get Postgres container port: {}", e);
    //         }
    //     };
    //     let database_url =
    //         format!("postgres://myuser:mypassword@127.0.0.1:{pg_port}/mydb?sslmode=disable");
    //     let pool = match PgPoolOptions::new()
    //         .max_connections(5)
    //         .acquire_timeout(Duration::from_secs(5))
    //         .connect(&database_url)
    //         .await
    //     {
    //         Ok(o) => o,
    //         Err(e) => {
    //             let _ = stdout()
    //                 .write_all(format!("Failed to connect to Postgres: {}\n", e).as_bytes())
    //                 .await;
    //             panic!("Failed to connect to Postgres: {}", e);
    //         }
    //     };
    //     sqlx::migrate!("./migrations")
    //         .run(&pool)
    //         .await
    //         .expect("run migrations");
    //     let redis_image = GenericImage::new("redis", "alpine3.23")
    //         .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));
    //     let redis_container = redis_image
    //         .start()
    //         .await
    //         .expect("Failed to start Redis container");
    //     let redis_host = redis_container.get_host().await.unwrap();
    //     let redis_port = match redis_container.get_host_port_ipv4(6379).await {
    //         Ok(c) => c,
    //         Err(e) => {
    //             let _ = stdout()
    //                 .write_all(format!("Failed to get Redis container port: {}\n", e).as_bytes())
    //                 .await;
    //             panic!("Failed to get Redis container port: {}", e);
    //         }
    //     };
    //     let redis_url = format!("redis://{redis_host}:{redis_port}");
    //     let redis_client = redis::Client::open(redis_url).expect("redis client");
    //     let redis = redis::aio::ConnectionManager::new(redis_client)
    //         .await
    //         .expect("redis conn manager");
    //     let state = Arc::new(AppState::new_for_tests(pool.clone(), redis.clone()));
    //     Self {
    //         _pg: pg_container,
    //         _redis: redis_container,
    //         pool,
    //         redis,
    //         state,
    //     }
    // }

    //

    pub async fn new() -> Arc<Self> {
        dotenvy::dotenv().ok();

        let pg_image = GenericImage::new("postgres", "16.15-alpine3.24")
            .with_wait_for(WaitFor::message_on_stdout(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_USER", "myuser")
            .with_env_var("POSTGRES_PASSWORD", "mypassword")
            .with_env_var("POSTGRES_DB", "mydb");

        let pg_container = pg_image
            .start()
            .await
            .expect("Failed to start Postgres container");

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let pg_port = match pg_container.get_host_port_ipv4(5432).await {
            Ok(c) => c,
            Err(e) => {
                let _ = stdout()
                    .write_all(format!("Failed to get Postgres container port: {}\n", e).as_bytes())
                    .await;
                panic!("Failed to get Postgres container port: {}", e);
            }
        };

        let database_url =
            format!("postgres://myuser:mypassword@127.0.0.1:{pg_port}/mydb?sslmode=disable");

        let mut pool_option: Option<PgPool> = None;
        let mut attempts = 0;
        let max_attempts = 7;

        while attempts < max_attempts {
            match PgPoolOptions::new()
                .max_connections(5)
                .acquire_timeout(std::time::Duration::from_secs(2))
                .connect(&database_url)
                .await
            {
                Ok(p) => {
                    pool_option = Some(p);
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    let _ = stdout()
                        .write_all(
                            format!(
                                "⚠️ [ step {}/{}] the error : 500 : {}\n",
                                attempts, max_attempts, e
                            )
                            .as_bytes(),
                        )
                        .await;
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }

        let pool = pool_option.unwrap_or_else(|| {
            panic!(
                "Failed to connect to Postgres after {} attempts",
                max_attempts
            );
        });

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        let redis_image = GenericImage::new("redis", "alpine3.23")
            .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

        let redis_container = redis_image
            .start()
            .await
            .expect("Failed to start Redis container");

        let redis_host = redis_container.get_host().await.unwrap();
        let redis_port = match redis_container.get_host_port_ipv4(6379).await {
            Ok(c) => c,
            Err(e) => {
                let _ = stdout()
                    .write_all(format!("Failed to get Redis container port: {}\n", e).as_bytes())
                    .await;
                panic!("Failed to get Redis container port: {}", e);
            }
        };
        let redis_url = format!("redis://{redis_host}:{redis_port}");

        let redis_client = redis::Client::open(redis_url).expect("redis client");
        let redis = redis::aio::ConnectionManager::new(redis_client)
            .await
            .expect("redis conn manager");

        let state = Arc::new(AppState::new_for_tests(pool.clone(), redis.clone()));

        Arc::new(Self {
            _pg: pg_container,
            _redis: redis_container,
            pool,
            redis,
            state,
        })
    }

    //

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

//   HTTP helpers

pub fn json_req(method: &str, uri: &str, body: Value) -> Request<Body> {
    match Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
    {
        Ok(r) => r,
        Err(e) => {
            panic!("Failed to build request: {}", e);
        }
    }
}

pub fn get_req(uri: &str, bearer: &str) -> Request<Body> {
    match Request::builder()
        .method("GET")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::empty())
    {
        Ok(r) => r,
        Err(e) => {
            panic!("Failed to build GET request: {}", e);
        }
    }
}

pub fn patch_req(uri: &str, bearer: &str, body: Value) -> Request<Body> {
    match Request::builder()
        .method("PATCH")
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"))
        .body(Body::from(body.to_string()))
    {
        Ok(r) => r,
        Err(e) => {
            panic!("Failed to build PATCH request: {}", e);
        }
    }
}

pub async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

pub async fn register_user(app: &axum::Router, email: &str) -> Value {
    use tower::ServiceExt;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_test_writer()
        .try_init();

    let resp = match app
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
    {
        Ok(r) => r,
        Err(e) => {
            panic!("Failed to send request: {}", e);
        }
    };

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    body_json(resp).await
}

//
