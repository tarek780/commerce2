mod common;

use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;

use common::{TestContext, json_req, register_user, truncate_all, DELETE_USER};


#[tokio::test]
async fn delete_user_removes_user_and_tokens() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();

    let email = "del@example.com";
    let v = register_user(&app, email).await;
    let user_id = v["user_id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            DELETE_USER,
            json!({ "user_id": user_id }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = $1")
        .bind(&user_id)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(count.0, 0);

    let tcount: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM refresh_tokens WHERE user_id = $1")
        .bind(&user_id)
        .fetch_one(&ctx.pool)
        .await
        .unwrap();
    assert_eq!(tcount.0, 0);
}