mod common;

use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;

use common::{TestContext, body_json, json_req, truncate_all, LOGIN, REGISTER};

#[tokio::test]
async fn register_success_returns_tokens() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let resp = app
        .oneshot(json_req(
            "POST",
            REGISTER,
            json!({
                "full_name": "Tarek",
                "username": "tarek",
                "u_role": "user",
                "email": "tarek@example.com",
                "password": "supersecret1",
                "display_name": "Tarek"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v["user_id"].is_string());
    assert!(v["tokens"]["access_token"].is_string());
    assert!(v["tokens"]["refresh_token"].is_string());
    assert_eq!(v["tokens"]["token_type"], "Bearer");
}

#[tokio::test]
async fn register_duplicate_email_conflict() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let payload = json!({
        "full_name": "X", "username": "x", "u_role": "user",
        "email": "dup@example.com", "password": "supersecret1"
    });

    let r1 = app
        .clone()
        .oneshot(json_req("POST", REGISTER, payload.clone()))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    let r2 = app
        .oneshot(json_req("POST", REGISTER, payload))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn register_invalid_email_rejected() {
    let ctx = TestContext::new().await;
    let app = ctx.router();

    let resp = app
        .oneshot(json_req(
            "POST",
            REGISTER,
            json!({
                "full_name": "X", "username": "x", "u_role": "user",
                "email": "not-an-email", "password": "supersecret1"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn register_short_password_rejected() {
    let ctx = TestContext::new().await;
    let app = ctx.router();

    let resp = app
        .oneshot(json_req(
            "POST",
            REGISTER,
            json!({
                "full_name": "X", "username": "x", "u_role": "user",
                "email": "short@example.com", "password": "123"
            }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn login_success() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    app.clone()
        .oneshot(json_req(
            "POST",
            REGISTER,
            json!({
                "full_name": "Sara", "username": "sara", "u_role": "user",
                "email": "sara@example.com", "password": "supersecret1"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_req(
            "POST",
            LOGIN,
            json!({ "email": "sara@example.com", "password": "supersecret1" }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp).await;
    assert!(v["tokens"]["access_token"].is_string());
}

#[tokio::test]
async fn login_wrong_password_unauthorized() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    app.clone()
        .oneshot(json_req(
            "POST",
            REGISTER,
            json!({
                "full_name": "Y", "username": "y", "u_role": "user",
                "email": "y@example.com", "password": "supersecret1"
            }),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(json_req(
            "POST",
            LOGIN,
            json!({ "email": "y@example.com", "password": "wrong-pass" }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn login_unknown_email_unauthorized() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let resp = app
        .oneshot(json_req(
            "POST",
            LOGIN,
            json!({ "email": "nobody@example.com", "password": "whatever1" }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}