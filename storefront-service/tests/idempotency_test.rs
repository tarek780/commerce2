mod common;
use common::{auth, fixtures::*, post_json, USER_ID};

#[tokio::test]
async fn duplicate_idempotency_key_returns_cached_response() {
    let h = common::harness().await;
    *h.cart.items.lock().await = vec![cart_item(1, 1)];
    h.category.with_prices(vec![(1, "20.00")]).await;

    let key = "test-key-123";
    let token = auth::access_token_for(USER_ID);
    let headers = [
        ("authorization", auth::bearer(&token)),
        ("idempotency-key", key.to_string()),
    ];
    let hdr: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.as_str())).collect();

    let (s1, j1, _) = post_json(&h.app, "/api/v1/checkout", &hdr, serde_json::json!({})).await;
    assert_eq!(s1, 201);

    let (s2, j2, _) = post_json(&h.app, "/api/v1/checkout", &hdr, serde_json::json!({})).await;
    assert_eq!(s2, 201);
    assert_eq!(j1["order_id"], j2["order_id"]);

    assert_eq!(*h.order.create_calls.lock().await, 1);
}