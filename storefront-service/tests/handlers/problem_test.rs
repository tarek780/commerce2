use crate::common::{TestHarness, USER_ID, auth, get_json};
use std::sync::Arc;

pub async fn problem_details_follows_rfc7807(h: &Arc<TestHarness>) {
    let token = auth::access_token_for(USER_ID);
    let (status, json, ct) = get_json(
        &h.app,
        "/api/v1/orders/missing",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;

    assert_eq!(status, 404);
    assert!(ct.unwrap().contains("application/problem+json"));
    assert_eq!(json["status"], 404);
    assert!(
        json["type"]
            .as_str()
            .unwrap()
            .starts_with("https://storefront.example.com/errors/")
    );
    assert!(json["title"].is_string());
    assert!(json["detail"].is_string());
}

pub async fn unauthorized_response_is_plain_401(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json(&h.app, "/api/v1/orders", &[]).await;
    assert_eq!(status, 401);
}

pub async fn problem_master(state: Arc<TestHarness>) {
    let h = &state;
    problem_details_follows_rfc7807(h).await;
    unauthorized_response_is_plain_401(h).await;
}
