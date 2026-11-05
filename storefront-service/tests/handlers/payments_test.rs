use crate::common::{TestHarness, USER_ID, auth, fixtures::*, get_json};
use std::sync::Arc;

async fn bearer_for(user: &str) -> Vec<(&'static str, String)> {
    let token = auth::token_gen("ACCESS", "USER", 99999).await;

    vec![("authorization", token)]
}

fn refs<'a>(a: &'a [(&'static str, String)]) -> Vec<(&'static str, &'a str)> {
    a.iter().map(|(k, v)| (*k, v.as_str())).collect()
}

pub async fn get_payment_success(h: &Arc<TestHarness>) {
    h.payment
        .payments
        .lock()
        .await
        .push(payment_response("pay-1"));
    let hdr = bearer_for(USER_ID).await;
    let (status, json, _) = get_json("/api/v1/payments/pay-1", &refs(&hdr)).await;
    assert_eq!(status, 200);
    assert_eq!(json["id"], "pay-1");
    assert_eq!(json["status"], "succeeded");
}

pub async fn get_payment_not_found(h: &Arc<TestHarness>) {
    let hdr = bearer_for(USER_ID).await;
    let (status, _, _) = get_json("/api/v1/payments/missing", &refs(&hdr)).await;
    assert_eq!(status, 404);
}

pub async fn get_payment_unauthenticated_returns_401(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json("/api/v1/payments/pay-1", &[]).await;
    assert_eq!(status, 401);
}

pub async fn payment_master(state: Arc<TestHarness>) {
    let h = state;
    get_payment_not_found(&h).await;
    get_payment_success(&h).await;
    get_payment_unauthenticated_returns_401(&h).await
}
