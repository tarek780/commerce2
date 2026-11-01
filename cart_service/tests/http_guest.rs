
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn http_guest_flow_create_add_get() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };
    let app = common::build_router(env.state.clone());

    // POST /api/guest/cart
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/guest/cart")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let guest_id = json["guest_cart_id"].as_str().unwrap().to_string();

    // POST /api/guest/cart/:id/items
    let payload = serde_json::json!({
        "sku_id": "SKU-HTTP",
        "quantity": 2,
        "price": 4.5
    });
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/guest/cart/{guest_id}/items"))
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/guest/cart/:id
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/guest/cart/{guest_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"][0]["sku_id"], "SKU-HTTP");
    assert_eq!(json["items"][0]["quantity"], 2);


    env.state
        .cart_service
        .guest_repo
        .delete(&guest_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn http_missing_guest_cart_returns_404() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };
    let app = common::build_router(env.state.clone());

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/guest/cart/nonexistent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}