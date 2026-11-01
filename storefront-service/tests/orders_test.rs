mod common;
use common::{auth, fixtures::*, get_json, post_json, USER_ID, OTHER_USER_ID};

fn bearer_for(user: &str) -> [(&'static str, String); 1] {
    [("authorization", auth::bearer(&auth::access_token_for(user)))]
}

fn refs(arr: &[(&'static str, String)]) -> Vec<(&str, &str)> {
    arr.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

#[tokio::test]
async fn list_orders_empty() {
    let h = common::harness().await;
    let hdr = bearer_for(USER_ID);
    let (status, json, _) = get_json(&h.app, "/api/v1/orders?first=10", &refs(&hdr)).await;

    assert_eq!(status, 200);
    assert_eq!(json["edges"].as_array().unwrap().len(), 0);
    assert_eq!(json["page_info"]["has_next_page"], false);
}

#[tokio::test]
async fn list_orders_with_cursor_pagination() {
    use storefront_service::clients::order::pb_order::ListOrdersResponse;
    let h = common::harness().await;

    let o1 = order_response("order-1", USER_ID);
    let o2 = order_response("order-2", USER_ID);
    *h.order.list_response.lock().await = Some(ListOrdersResponse {
        edges: vec![o1, o2],
        has_next_page: true,
        end_cursor: "cursor2".into(),
    });

    let hdr = bearer_for(USER_ID);
    let (status, json, _) = get_json(&h.app, "/api/v1/orders?first=2", &refs(&hdr)).await;

    assert_eq!(status, 200);
    assert_eq!(json["edges"].as_array().unwrap().len(), 2);
    assert_eq!(json["page_info"]["has_next_page"], true);
    assert_eq!(json["page_info"]["end_cursor"], "cursor2");
    assert_eq!(json["edges"][0]["node"]["id"], "order-1");
}

#[tokio::test]
async fn get_order_success() {
    let h = common::harness().await;
    h.order.orders.lock().await.push(order_response("order-1", USER_ID));
    let hdr = bearer_for(USER_ID);
    let (status, json, _) = get_json(&h.app, "/api/v1/orders/order-1", &refs(&hdr)).await;
    assert_eq!(status, 200);
    assert_eq!(json["id"], "order-1");
}

#[tokio::test]
async fn get_order_not_owner_returns_404() {
    let h = common::harness().await;
    h.order.orders.lock().await.push(order_response("order-1", OTHER_USER_ID));
    let hdr = bearer_for(USER_ID);
    let (status, _, _) = get_json(&h.app, "/api/v1/orders/order-1", &refs(&hdr)).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn get_order_not_found() {
    let h = common::harness().await;
    let hdr = bearer_for(USER_ID);
    let (status, _, _) = get_json(&h.app, "/api/v1/orders/unknown", &refs(&hdr)).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn cancel_order_success() {
    let h = common::harness().await;
    h.order.orders.lock().await.push(order_response("order-1", USER_ID));
    let hdr = bearer_for(USER_ID);
    let (status, json, _) = post_json(
        &h.app,
        "/api/v1/orders/order-1/cancel",
        &refs(&hdr),
        serde_json::json!({ "reason": "customer_request" }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(json["status"], "cancelled");
}

#[tokio::test]
async fn cancel_order_not_owner_returns_404() {
    let h = common::harness().await;
    h.order.orders.lock().await.push(order_response("order-1", OTHER_USER_ID));
    let hdr = bearer_for(USER_ID);
    let (status, _, _) = post_json(
        &h.app,
        "/api/v1/orders/order-1/cancel",
        &refs(&hdr),
        serde_json::json!({ "reason": "x" }),
    )
    .await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn pay_order_success() {
    let h = common::harness().await;
    h.order.orders.lock().await.push(order_response("order-1", USER_ID));
    *h.order.pay_response.lock().await = Some(pay_response("order-1"));
    let hdr = bearer_for(USER_ID);
    let (status, json, _) = post_json(
        &h.app,
        "/api/v1/orders/order-1/pay",
        &refs(&hdr),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(json["payment_url"], "http://pay/mock-1");
    assert_eq!(json["status"], "processing");
}