mod common;
use common::{auth, fixtures::*, get_json, USER_ID};

fn bearer_for(user: &str) -> [(&'static str, String); 1] {
    [("authorization", auth::bearer(&auth::access_token_for(user)))]
}

fn refs(a: &[(&'static str, String)]) -> Vec<(&str, &str)> {
    a.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

#[tokio::test]
async fn get_payment_success() {
    let h = common::harness().await;
    h.payment.payments.lock().await.push(payment_response("pay-1"));
    let hdr = bearer_for(USER_ID);
    let (status, json, _) = get_json(&h.app, "/api/v1/payments/pay-1", &refs(&hdr)).await;
    assert_eq!(status, 200);
    assert_eq!(json["id"], "pay-1");
    assert_eq!(json["status"], "succeeded");
}

#[tokio::test]
async fn get_payment_not_found() {
    let h = common::harness().await;
    let hdr = bearer_for(USER_ID);
    let (status, _, _) = get_json(&h.app, "/api/v1/payments/missing", &refs(&hdr)).await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn get_payment_unauthenticated_returns_401() {
    let h = common::harness().await;
    let (status, _, _) = get_json(&h.app, "/api/v1/payments/pay-1", &[]).await;
    assert_eq!(status, 401);
}