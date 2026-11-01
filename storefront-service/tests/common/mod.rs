#![allow(dead_code)]

pub mod auth;
pub mod fixtures;
pub mod mocks;

use std::sync::Arc;
use axum::Router;

use storefront_service::config::AppConfig;
use storefront_service::state::AppState;

use self::mocks::cart::MockCartClient;
use self::mocks::category::MockCategoryClient;
use self::mocks::order::MockOrderClient;
use self::mocks::payment::MockPaymentClient;

use testcontainers::{
    ContainerAsync,
    core::{Image, ImageExt, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use testcontainers::{ContainerRequest, GenericImage};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;

pub fn test_config() -> AppConfig {
    AppConfig {
        http_addr: "127.0.0.1:0".into(),
        currency: "USD".into(),
        cart_base_url: "http://unused".into(),
        cart_clear_endpoint: "/api/cart".into(),
        category_graphql_url: "http://unused".into(),
        order_grpc_url: "http://unused".into(),
        payment_grpc_url: "http://unused".into(),
        inventory_grpc_url: "http://unused".into(),
        redis_url: "redis://unused".into(),
        idempotency_ttl_secs: 3600,
        idempotency_required: false,
        upstream_timeout: std::time::Duration::from_secs(5),
    }
}

pub struct TestHarness {
    pub app: Router,
    pub cart: Arc<MockCartClient>,
    pub category: Arc<MockCategoryClient>,
    pub order: Arc<MockOrderClient>,
    pub payment: Arc<MockPaymentClient>,
    pub idempotency: Arc<storefront_service::helpers::idempotency::MemoryIdempotencyStore>,
}

pub async fn harness() -> TestHarness {
    auth::init_secret_once();

    let cart = Arc::new(MockCartClient::new());
    let category = Arc::new(MockCategoryClient::new());
    let order = Arc::new(MockOrderClient::new());
    let payment = Arc::new(MockPaymentClient::new());
    let idempotency =
        Arc::new(storefront_service::helpers::idempotency::MemoryIdempotencyStore::new());

    let state = AppState::for_tests(
        test_config(),
        cart.clone(),
        category.clone(),
        order.clone(),
        payment.clone(),
        idempotency.clone(),
    );

    let app = storefront_service::routes::build(Arc::new(state));
    TestHarness {
        app,
        cart,
        category,
        order,
        payment,
        idempotency,
    }
}

//  HTTP helpers

pub async fn post_json(
    app: &Router,
    path: &str,
    headers: &[(&str, &str)],
    body: serde_json::Value,
) -> (u16, serde_json::Value, Option<String>) {
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");

    for (k, v) in headers {
        req = req.header(*k, *v);
    }

    let req = req.body(Body::from(serde_json::to_vec(&body).unwrap())).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status.as_u16(), json, ct)
}

pub async fn get_json(
    app: &Router,
    path: &str,
    headers: &[(&str, &str)],
) -> (u16, serde_json::Value, Option<String>) {
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let mut req = Request::builder().method("GET").uri(path);
    for (k, v) in headers {
        req = req.header(*k, *v);
    }

    let req = req.body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status.as_u16(), json, ct)
}


        unsafe {
            std::env::set_var("SECRET_KEY", "test-secret-key-32-chars-minimum-xx");
            std::env::set_var("RUST_LOG", "info");
            std::env::set_var("RUST_BACKTRACE", "1");
        }

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

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

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

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("connect test postgres");

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
        //


pub const USER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
pub const OTHER_USER_ID: &str = "11111111-1111-1111-1111-111111111111";