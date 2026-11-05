use uuid::Uuid;

use crate::common::{TestHarness, USER_ID, auth, fixtures::*, post_json};
use std::sync::Arc;
use storefront_service::error::AppError;

fn idem_headers_for(user: &str) -> Vec<(String, String)> {
    vec![
        (
            "authorization".into(),
            auth::bearer(&auth::access_token_for(user)),
        ),
        ("idempotency-key".into(), uuid::Uuid::new_v4().to_string()),
    ]
}

fn as_refs(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter()
        .map(|(k, val)| (k.as_str(), val.as_str()))
        .collect()
}

pub async fn checkout_success(h: &Arc<TestHarness>) {
    let id = Uuid::now_v7();
    let id2 = Uuid::now_v7();
    *h.cart.items.lock().await = vec![cart_item(id, 2), cart_item(id2, 1)];
    h.category
        .with_prices(vec![(id, "20.00"), (id2, "20.00")])
        .await;

    let headers = idem_headers_for(USER_ID);
    let (status, json, _) = post_json(
        &h.app,
        "/api/v1/checkout",
        &as_refs(&headers),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(status, 201);
    assert_eq!(json["order_id"], "order-1");
    assert_eq!(json["payment_id"], "pay-1");
    assert_eq!(json["warnings"].as_array().unwrap().len(), 0);
    assert_eq!(*h.order.create_calls.lock().await, 1);
    assert_eq!(*h.cart.clear_calls.lock().await, 1);
}

pub async fn checkout_empty_cart_returns_400(h: &Arc<TestHarness>) {
    let headers = idem_headers_for(USER_ID);
    let (status, json, ct) = post_json(
        &h.app,
        "/api/v1/checkout",
        &as_refs(&headers),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(status, 400);
    assert!(ct.unwrap().contains("application/problem+json"));
    assert!(json["detail"].as_str().unwrap().contains("empty cart"));
}

pub async fn checkout_insufficient_stock_maps_to_402(h: &Arc<TestHarness>) {
    let id = Uuid::now_v7();
    *h.cart.items.lock().await = vec![cart_item(id, 5)];
    h.category.with_prices(vec![(id, "20.00")]).await;
    *h.order.create_response.lock().await = Some(Err(AppError::InsufficientStock {
        variant_id: 0,
        requested: 5,
        available: 3,
    }));

    let headers = idem_headers_for(USER_ID);
    let (status, json, _) = post_json(
        &h.app,
        "/api/v1/checkout",
        &as_refs(&headers),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(status, 402);
    assert_eq!(
        json["type"],
        "https://storefront.example.com/errors/insufficient-stock"
    );
    assert_eq!(*h.cart.clear_calls.lock().await, 0);
}

pub async fn checkout_invalid_quantity_rejected(h: &Arc<TestHarness>) {
    let id = Uuid::now_v7();
    *h.cart.items.lock().await = vec![cart_item(id, 0)];
    h.category.with_prices(vec![(id, "20.00")]).await;

    let headers = idem_headers_for(USER_ID);
    let (status, _, _) = post_json(
        &h.app,
        "/api/v1/checkout",
        &as_refs(&headers),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(status, 400);
    assert_eq!(*h.order.create_calls.lock().await, 0);
}

pub async fn checkout_category_failure_maps_to_502(h: &Arc<TestHarness>) {
    let id = Uuid::now_v7();
    *h.cart.items.lock().await = vec![cart_item(id, 1)];
    h.category.fail_for(id).await;

    let headers = idem_headers_for(USER_ID);
    let (status, _, _) = post_json(
        &h.app,
        "/api/v1/checkout",
        &as_refs(&headers),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(status, 502);
}

pub async fn checkout_cart_clear_failure_returns_warnings(h: &Arc<TestHarness>) {
    let id = Uuid::now_v7();
    *h.cart.items.lock().await = vec![cart_item(id, 1)];
    h.category.with_prices(vec![(id, "20.00")]).await;
    h.cart.set_fail_clear(true).await;

    let headers = idem_headers_for(USER_ID);
    let (status, json, _) = post_json(
        &h.app,
        "/api/v1/checkout",
        &as_refs(&headers),
        serde_json::json!({}),
    )
    .await;

    assert_eq!(status, 201);
    let warnings = json["warnings"].as_array().unwrap();
    assert!(warnings.contains(&serde_json::json!("cart_not_cleared")));
}

pub async fn checkout_master(state: Arc<TestHarness>) {
    let h = state;
    checkout_success(&h).await;
    checkout_empty_cart_returns_400(&h).await;
    checkout_cart_clear_failure_returns_warnings(&h).await;
    checkout_category_failure_maps_to_502(&h).await;
    checkout_invalid_quantity_rejected(&h).await;
    checkout_insufficient_stock_maps_to_402(&h).await;
}
