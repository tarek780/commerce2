use auth_service::middleware::auth_layer::{AccessToken, AuthLayer};
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn echo_user(
    axum::extract::Extension(token): axum::extract::Extension<AccessToken>,
) -> String {
    token.user_id
}

fn build_app_with_layer() -> Router {
    let protected = Router::new()
        .route("/p", get(echo_user))
        .layer(AuthLayer {});

    Router::new().nest("/api", protected)
}

pub async fn rejects_missing_authorization() {
    let app = build_app_with_layer();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/p")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

pub async fn rejects_malformed_header() {
    let app = build_app_with_layer();

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/p")
                .header(header::AUTHORIZATION, "Token abc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

pub async fn accepts_valid_token_and_injects_user_id() {
    dotenvy::dotenv().ok().expect("var err");

    let token =
        auth_service::helpers::jwt::issue_token("user-123".into(), "user", "access", 3600).unwrap();

    let app = build_app_with_layer();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/p")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let body = String::from_utf8_lossy(&bytes);
    assert_eq!(body, "user-123");
}

pub async fn rejects_expired_token() {
    dotenvy::dotenv().ok().expect("var err");
    let token =
        auth_service::helpers::jwt::issue_token("user-123".into(), "user", "access", -1).unwrap();

    let app = build_app_with_layer();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/p")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
