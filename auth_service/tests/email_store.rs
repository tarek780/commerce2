mod common;

use auth_service::error::AppError;
use auth_service::helpers::email_store;
use common::TestContext;

#[tokio::test]
async fn issue_and_consume_roundtrip() {
    let ctx = TestContext::new().await;
    let email = "rt@example.com";

    let token = email_store::issue_token(&ctx.redis, email, 600, 60)
        .await
        .expect("issue");

    let got = email_store::consume_token(&ctx.redis, &token)
        .await
        .expect("consume");
    assert_eq!(got, email);
}

#[tokio::test]
async fn consume_removes_token() {
    let ctx = TestContext::new().await;
    let email = "once@example.com";

    let token = email_store::issue_token(&ctx.redis, email, 600, 60)
        .await
        .unwrap();
    email_store::consume_token(&ctx.redis, &token)
        .await
        .unwrap();

    let res = email_store::consume_token(&ctx.redis, &token).await;
    assert!(matches!(res, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn rate_limit_blocks_second_issue() {
    let ctx = TestContext::new().await;
    let email = "rl2@example.com";

    email_store::issue_token(&ctx.redis, email, 600, 60)
        .await
        .unwrap();
    let res = email_store::issue_token(&ctx.redis, email, 600, 60).await;
    assert!(matches!(res, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn token_expires_after_ttl() {
    let ctx = TestContext::new().await;
    let email = "ttl@example.com";

    let token = email_store::issue_token(&ctx.redis, email, 1, 1)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let res = email_store::consume_token(&ctx.redis, &token).await;
    assert!(matches!(res, Err(AppError::Validation(_))));
}
