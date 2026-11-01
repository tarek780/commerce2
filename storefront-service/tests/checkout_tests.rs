mod common;
use common::{auth, fixtures::*, harness, post_json, USER_ID};
use storefront_service::error::AppError;


fn idem_headers_for(user: &str) -> Vec<(String, String)> {
    vec![
        ("authorization".into(), auth::bearer(&auth::access_token_for(user))),
        ("idempotency-key".into(), uuid::Uuid::new_v4().to_string()),
    ]
}

fn as_refs(v: &[(String, String)]) -> Vec<(&str, &str)> {
    v.iter().map(|(k, val)| (k.as_str(), val.as_str())).collect()
}

#[tokio::test]
async fn checkout_success() {
    let h = harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 2), cart_item(2, 1)];
    h.category.with_prices(vec![(1, "20.00"), (2, "20.00")]).await;

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

#[tokio::test]
async fn checkout_empty_cart_returns_400() {
    let h = harness().await;
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

#[tokio::test]
async fn checkout_insufficient_stock_maps_to_402() {
    let h = harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 5)];
    h.category.with_prices(vec![(1, "20.00")]).await;
    *h.order.create_response.lock().await = Some(Err(AppError::InsufficientStock {
        variant_id: 1,
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

#[tokio::test]
async fn checkout_invalid_quantity_rejected() {
    let h = harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 0)];
    h.category.with_prices(vec![(1, "20.00")]).await;

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

#[tokio::test]
async fn checkout_category_failure_maps_to_502() {
    let h = harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 1)];
    h.category.fail_for(1).await;

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

#[tokio::test]
async fn checkout_cart_clear_failure_returns_warnings() {
    let h = harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 1)];
    h.category.with_prices(vec![(1, "20.00")]).await;
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