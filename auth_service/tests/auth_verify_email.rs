mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use common::{
    TestContext, VERIFY_EMAIL, body_json, issue_email_token_for, register_user, truncate_all,
};

#[tokio::test]
async fn verify_email_with_valid_token_marks_verified() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let email = "verify@example.com";
    register_user(&app, email).await;

    use redis::AsyncCommands;
    let mut conn = ctx.redis.clone();
    let _: () = conn.del(format!("email_verify_rl:{email}")).await.unwrap();

    let vtoken = issue_email_token_for(&ctx.redis, email).await;

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("{VERIFY_EMAIL}?token={vtoken}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert_eq!(v["email"], email);

    let row: (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT email_verified_at FROM users WHERE email = $1")
            .bind(email)
            .fetch_one(&ctx.pool)
            .await
            .unwrap();
    assert!(row.0.is_some(), "email_verified_at يجب أن يكون مضبوطاً");
}

#[tokio::test]
async fn verify_email_token_is_single_use() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let email = "once@example.com";
    register_user(&app, email).await;

    use redis::AsyncCommands;
    let mut conn = ctx.redis.clone();
    let _: () = conn.del(format!("email_verify_rl:{email}")).await.unwrap();

    let vtoken = issue_email_token_for(&ctx.redis, email).await;

    let r1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("{VERIFY_EMAIL}?token={vtoken}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    let r2 = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("{VERIFY_EMAIL}?token={vtoken}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(r2.status().is_client_error());
}

#[tokio::test]
async fn verify_email_with_invalid_token_rejected() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("{VERIFY_EMAIL}?token=does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn verify_email_rate_limit_blocks_second_issue() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;

    let email = "rl@example.com";
    let _t1 = issue_email_token_for(&ctx.redis, email).await;

    let res = auth_service::helpers::email_store::issue_token(&ctx.redis, email, 600, 60).await;
    assert!(res.is_err());
}

// #[allow(dead_code)]
// fn _unused(_: serde_json::Value) {
//     let _ = json;
//     let _ = json_req;
// }
