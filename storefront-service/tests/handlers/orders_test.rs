use crate::common::{OTHER_USER_ID, TestHarness, USER_ID, auth, fixtures::*, get_json, post_json};
use std::sync::Arc;

async fn bearer_for(user: &str) -> Vec<(&'static str, String)> {
    let token = auth::token_gen("ACCESS", "USER", 99999).await;
    vec![("authorization", token)]
}

fn refs<'a>(a: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    a.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

pub async fn list_orders_empty(h: &Arc<TestHarness>) {
    let hdr = bearer_for(USER_ID).await;
    let (status, json, _) = get_json("/api/v1/orders?first=10", &refs(&hdr)).await;

    assert_eq!(status, 200);
    assert_eq!(json["edges"].as_array().unwrap().len(), 0);
    assert_eq!(json["page_info"]["has_next_page"], false);
}

// pub async fn list_orders_with_cursor_pagination(h: &Arc<TestHarness>) {
//     use storefront_service::clients::order::pb_order::ListOrdersResponse;
//     let o1 = order_response("order-1", USER_ID);
//     let o2 = order_response("order-2", USER_ID);
//     *h.order.list_response.lock().await = Some(ListOrdersResponse {
//         edges: vec![o1, o2],
//         has_next_page: true,
//         end_cursor: "cursor2".into(),
//     });
//     let hdr = bearer_for(USER_ID).await;
//     let (status, json, _) = get_json( "/api/v1/orders?first=2", &refs(&hdr)).await;
//     assert_eq!(status, 200);
//     assert_eq!(json["edges"].as_array().unwrap().len(), 2);
//     assert_eq!(json["page_info"]["has_next_page"], true);
//     assert_eq!(json["page_info"]["end_cursor"], "cursor2");
//     assert_eq!(json["edges"][0]["node"]["id"], "order-1");
// }

pub async fn get_order_success(h: &Arc<TestHarness>) {
    h.order
        .orders
        .lock()
        .await
        .push(order_response("order-1", USER_ID));
    let hdr = bearer_for(USER_ID).await;
    let (status, json, _) = get_json("/api/v1/orders/order-1", &refs(&hdr)).await;
    assert_eq!(status, 200);
    assert_eq!(json["id"], "order-1");
}

pub async fn get_order_not_owner_returns_404(h: &Arc<TestHarness>) {
    h.order
        .orders
        .lock()
        .await
        .push(order_response("order-1", OTHER_USER_ID));
    let hdr = bearer_for(USER_ID).await;
    let (status, _, _) = get_json("/api/v1/orders/order-1", &refs(&hdr)).await;
    assert_eq!(status, 404);
}

pub async fn get_order_not_found(h: &Arc<TestHarness>) {
    let hdr = bearer_for(USER_ID).await;
    let (status, _, _) = get_json("/api/v1/orders/unknown", &refs(&hdr)).await;
    assert_eq!(status, 404);
}

pub async fn cancel_order_success(h: &Arc<TestHarness>) {
    h.order
        .orders
        .lock()
        .await
        .push(order_response("order-1", USER_ID));
    let hdr = bearer_for(USER_ID).await;
    let (status, json, _) = post_json(
        "/api/v1/orders/order-1/cancel",
        &refs(&hdr),
        serde_json::json!({ "reason": "customer_request" }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(json["status"], "cancelled");
}

pub async fn cancel_order_not_owner_returns_404(h: &Arc<TestHarness>) {
    h.order
        .orders
        .lock()
        .await
        .push(order_response("order-1", OTHER_USER_ID));
    let hdr = bearer_for(USER_ID).await;
    let (status, _, _) = post_json(
        "/api/v1/orders/order-1/cancel",
        &refs(&hdr),
        serde_json::json!({ "reason": "x" }),
    )
    .await;
    assert_eq!(status, 404);
}

pub async fn pay_order_success(h: &Arc<TestHarness>) {
    h.order
        .orders
        .lock()
        .await
        .push(order_response("order-1", USER_ID));
    *h.order.pay_response.lock().await = Some(pay_response("order-1"));
    let hdr = bearer_for(USER_ID).await;
    let (status, json, _) = post_json(
        "/api/v1/orders/order-1/pay",
        &refs(&hdr),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(json["payment_url"], "http://pay/mock-1");
    assert_eq!(json["status"], "processing");
}

pub async fn order_master(state: Arc<TestHarness>) {
    let h = state;
    list_orders_empty(&h).await;
    get_order_not_found(&h).await;
    get_order_success(&h).await;
    get_order_not_owner_returns_404(&h).await;
    cancel_order_not_owner_returns_404(&h).await;
    cancel_order_success(&h).await;
    pay_order_success(&h).await;
}
