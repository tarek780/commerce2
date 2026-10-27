mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use tower::ServiceExt;

use common::{TestContext, truncate_all};

#[tokio::test]
async fn register_still_succeeds_when_smtp_fails() {
    let ctx = TestContext::new().await;
    truncate_all(&ctx.pool).await;
    let app = ctx.router();



    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "full_name": "S", "username": "s", "u_role": "user",
                        "email": "smtp@example.com", "password": "supersecret1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}