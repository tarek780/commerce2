use std::sync::Arc;

use crate::common::*;
use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;
pub async fn delete_user_removes_user_and_tokens(ctx: Arc<TestContext>) {
    let app = ctx.router();

    let email = "del@example.com";
    let v = register_user(&app, email).await;
    let user_id = v["user_id"].as_str().unwrap().to_string();
    let user_id_uuid = uuid::Uuid::parse_str(&user_id).unwrap();

    let resp = app
        .clone()
        .oneshot(json_req("POST", DELETE_USER, json!({ "user_id": user_id })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(&user_id_uuid)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    let tcount: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
        .bind(&user_id_uuid)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(tcount.0, 0);
}
