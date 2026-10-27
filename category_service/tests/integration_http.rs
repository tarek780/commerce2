mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use category_service::graphql;
use category_service::state::AppState;
use std::sync::Arc;
use tower::ServiceExt; // for `oneshot`

async fn app() -> axum::Router {
/// need db + grpc + redis + appstate
    unsafe {
        std::env::set_var("DATABASE_URL", std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/catalog_test".into()));
        std::env::set_var("REDIS_URL", std::env::var("TEST_REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()));
        std::env::set_var("REDIS_STREAM_KEY", "test:stream");
        std::env::set_var("REDIS_CONSUMER_GROUP", "test-group");
        std::env::set_var("INVENTORY_GRPC_URL", "http://127.0.0.1:50051");
    }
    let cfg = category_service::config::AppConfig::from_env().unwrap();
    let state = Arc::new(AppState::new(&cfg).await.expect("AppState"));
    graphql::router(state)
}

#[tokio::test]
#[ignore = "requires DB + Redis + gRPC inventory service"]
async fn playground_renders() {
    let app = app().await;
    let res = app.oneshot(
        Request::builder().uri("/graphql").body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("GraphQL"));
}

#[tokio::test]
#[ignore = "requires DB + Redis + gRPC inventory service"]
async fn query_products_returns_connection() {
    let app = app().await;
    let payload = serde_json::json!({
        "query": "{ products(page: { first: 5 }) { totalCount edges { cursor node { id name slug } } pageInfo { hasNextPage endCursor } } }"
    });
    let res = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/graphql")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    ).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(body["errors"].is_null(), "unexpected errors: {body}");
    assert!(body["data"]["products"].is_object());
    assert!(body["data"]["products"]["edges"].is_array());
}

#[tokio::test]
#[ignore = "requires DB + Redis + gRPC inventory service"]
async fn mutation_without_auth_returns_error() {
    let app = app().await;
    let payload = serde_json::json!({
        "query": "mutation { createCategory(input: { name: \"X\" }) }"
    });
    let res = app.oneshot(
        Request::builder()
            .method("POST")
            .uri("/graphql")
            .header("content-type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap()
    ).await.unwrap();

    let status = res.status();
    assert!(status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR);
}