mod common;

use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;

use common::{TestContext, body_json, json_req, register_user, truncate_all, REFRESH, LOGOUT};

#[tokio::test]
async fn refresh_rotates_token() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let v = register_user(&app, "rot@example.com").await;
    let old_refresh = v["tokens"]["refresh_token"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            REFRESH,
            json!({ "refresh_token": old_refresh }),
        ))
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let v2 = body_json(resp).await;
    let new_refresh = v2["refresh_token"].as_str().unwrap().to_string();

    assert_ne!(old_refresh, new_refresh, "refresh token يجب أن يُدوَّر");
    assert!(v2["access_token"].is_string());
}

#[tokio::test]
async fn refresh_reuse_detects_theft_and_revokes_family() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let v = register_user(&app, "theft@example.com").await;
    let old_refresh = v["tokens"]["refresh_token"].as_str().unwrap().to_string();

    let r1 = app
        .clone()
        .oneshot(json_req(
            "POST",
            REFRESH,
            json!({ "refresh_token": old_refresh }),
        ))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);
    let v2 = body_json(r1).await;
    let new_refresh = v2["refresh_token"].as_str().unwrap().to_string();

    let r2 = app
        .clone()
        .oneshot(json_req(
            "POST",
            REFRESH,
            json!({ "refresh_token": old_refresh }),
        ))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::UNAUTHORIZED);

    let r3 = app
        .oneshot(json_req(
            "POST",
            REFRESH,
            json!({ "refresh_token": new_refresh }),
        ))
        .await
        .unwrap();
    assert_eq!(r3.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_invalid_token_unauthorized() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let resp = app
        .oneshot(json_req(
            "POST",
            REFRESH,
            json!({ "refresh_token": "garbage-token" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_revokes_refresh() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let v = register_user(&app, "logout@example.com").await;
    let refresh = v["tokens"]["refresh_token"].as_str().unwrap().to_string();

    let r1 = app
        .clone()
        .oneshot(json_req(
            "POST",
            LOGOUT,
            json!({ "refresh_token": refresh }),
        ))
        .await
        .unwrap();
    assert_eq!(r1.status(), StatusCode::OK);

    let r2 = app
        .oneshot(json_req(
            "POST",
            REFRESH,
            json!({ "refresh_token": refresh }),
        ))
        .await
        .unwrap();
    assert_eq!(r2.status(), StatusCode::UNAUTHORIZED);
}