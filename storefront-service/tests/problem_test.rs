mod common;
use common::{auth, get_json, USER_ID};

#[tokio::test]
async fn problem_details_follows_rfc7807() {
    let h = common::harness().await;
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
    assert!(json["type"]
        .as_str()
        .unwrap()
        .starts_with("https://storefront.example.com/errors/"));
    assert!(json["title"].is_string());
    assert!(json["detail"].is_string());
}

#[tokio::test]
async fn unauthorized_response_is_plain_401() {

    let h = common::harness().await;
    let (status, _, _) = get_json(&h.app, "/api/v1/orders", &[]).await;
    assert_eq!(status, 401);
}