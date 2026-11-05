use crate::common::*;
use axum::http::StatusCode;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

pub async fn register_success_returns_tokens(ctx: Arc<TestContext>) {
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

pub async fn register_duplicate_email_conflict(ctx: Arc<TestContext>) {
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

pub async fn register_invalid_email_rejected(ctx: Arc<TestContext>) {
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

pub async fn register_short_password_rejected(ctx: Arc<TestContext>) {
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

pub async fn login_success(ctx: Arc<TestContext>) {
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

pub async fn login_wrong_password_unauthorized(ctx: Arc<TestContext>) {
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

pub async fn login_unknown_email_unauthorized(ctx: Arc<TestContext>) {
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
