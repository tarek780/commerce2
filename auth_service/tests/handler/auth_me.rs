use crate::common::*;
#[allow(dead_code)]
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

//
pub async fn get_me_returns_current_user(ctx: Arc<TestContext>) {
    let app = ctx.router();

    let v = register_user(&app, "me@example.com").await;
    let access = v["tokens"]["access_token"].as_str().unwrap().to_string();

    let resp = app.oneshot(get_req("/me", &access)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let me = body_json(resp).await;
    assert_eq!(me["email"], "me@example.com");
    assert_eq!(me["display_name"], "Initial");
    assert_eq!(me["email_verified"], false);
}

pub async fn get_me_without_token_unauthorized(ctx: Arc<TestContext>) {
    let resp = ctx
        .router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

pub async fn get_me_with_malformed_bearer_unauthorized(ctx: Arc<TestContext>) {
    let resp = ctx
        .router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/me")
                .header(header::AUTHORIZATION, "Token abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

//
pub async fn get_me_with_expired_access_token_unauthorized(ctx: Arc<TestContext>) {
    let app = ctx.router();

    let v = register_user(&app, "exp@example.com").await;
    let user_id = v["user_id"].as_str().unwrap().to_string();

    let expired = auth_service::helpers::jwt::issue_token(&user_id, "user", "access", -1)
        .expect("issue expired token");

    let resp = app.oneshot(get_req(ME, &expired)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

pub async fn get_me_with_forged_token_unauthorized(ctx: Arc<TestContext>) {
    let app = ctx.router();

    let resp = app.oneshot(get_req(ME, "aaa.bbb.ccc")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

pub async fn update_me_display_name(ctx: Arc<TestContext>) {
    let app = ctx.router();

    let v = register_user(&app, "upd@example.com").await;
    let access = v["tokens"]["access_token"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(patch_req(
            ME,
            &access,
            json!({ "display_name": "New Name" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let me = body_json(resp).await;
    assert_eq!(me["display_name"], "New Name");
}

//
pub async fn update_me_empty_display_name_rejected(ctx: Arc<TestContext>) {
    let app = ctx.router();

    let v = register_user(&app, "empty@example.com").await;
    let access = v["tokens"]["access_token"].as_str().unwrap().to_string();

    let resp = app
        .oneshot(patch_req(ME, &access, json!({ "display_name": "" })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

//
// fn _unused(_: Request<Body>) {
//     let _ = json_req;
// }
