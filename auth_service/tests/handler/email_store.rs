use crate::common::TestContext;
use auth_service::error::AppError;
use auth_service::helpers::email_store;
use std::sync::Arc;
pub async fn issue_and_consume_roundtrip(ctx: Arc<TestContext>) {
    let email = "rt@example.com";

    let token = email_store::issue_token(&ctx.redis, email, 600, 60)
        .await
        .expect("issue");

    let got = email_store::consume_token(&ctx.redis, &token)
        .await
        .expect("consume");
    assert_eq!(got, email);
}

pub async fn consume_removes_token(ctx: Arc<TestContext>) {
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

pub async fn rate_limit_blocks_second_issue(ctx: Arc<TestContext>) {
    let email = "rl2@example.com";

    email_store::issue_token(&ctx.redis, email, 600, 60)
        .await
        .unwrap();
    let res = email_store::issue_token(&ctx.redis, email, 600, 60).await;
    assert!(matches!(res, Err(AppError::Conflict(_))));
}

pub async fn token_expires_after_ttl(ctx: Arc<TestContext>) {
    let email = "ttl@example.com";

    let token = email_store::issue_token(&ctx.redis, email, 1, 1)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let res = email_store::consume_token(&ctx.redis, &token).await;
    assert!(matches!(res, Err(AppError::Validation(_))));
}
