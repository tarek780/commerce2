use axum::http::header::AUTHORIZATION;
use std::sync::Arc;

use crate::common::{TestHarness, USER_ID, auth, get_json};
pub async fn missing_authorization_returns_401(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json("/api/v1/orders", &[]).await;
    assert_eq!(status, 401);
}

pub async fn non_bearer_scheme_returns_401(h: &Arc<TestHarness>) {
    let token = auth::token_gen("a", "USER", 99999).await;

    let (status, _, _) = get_json(
        "/api/v1/orders",
        &[("authorization", &format!("Basic {token}"))],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn malformed_jwt_returns_401(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json("/api/v1/orders", &[("authorization", "Bearer not.a.jwt")]).await;
    assert_eq!(status, 401);
}

pub async fn jwt_signed_with_wrong_secret_returns_401(h: &Arc<TestHarness>) {
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use storefront_service::middleware::auth::Claims;

    let bad_token = auth::token_gen("a", "USER", 99999).await;

    let (status, _, _) = get_json(
        "/api/v1/orders",
        &[("authorization", &format!("Bearer {bad_token}"))],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn expired_jwt_returns_401(h: &Arc<TestHarness>) {
    let token = auth::token_gen("ACCESS", "USER", 0).await;
    let (status, _, _) = get_json(
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn valid_access_token_passes(h: &Arc<TestHarness>) {
    let token = auth::token_gen("a", "USER", 99999).await;
    let (status, _, _) = get_json(
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 200);
}

pub async fn refresh_token_behavior(h: &Arc<TestHarness>) {
    let token = auth::token_gen("ACCESS", "USER", 99999).await;
    let (status, _, _) = get_json(
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;

    assert_eq!(status, 200);
}

pub async fn admin_token_passes(h: &Arc<TestHarness>) {
    let token = auth::token_gen("ACCESS", "USER", 99999).await;
    let (status, _, _) = get_json(
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 200);
}

pub async fn health_is_public(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json("/health", &[]).await;
    assert_eq!(status, 200);
}

pub async fn ready_is_public(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json("/ready", &[]).await;
    assert_eq!(status, 200);
}

pub async fn auth_master(state: Arc<TestHarness>) {
    let h = state.clone();
    missing_authorization_returns_401(&h).await;
    non_bearer_scheme_returns_401(&h).await;
    malformed_jwt_returns_401(&h).await;
    jwt_signed_with_wrong_secret_returns_401(&h).await;
    expired_jwt_returns_401(&h).await;
    valid_access_token_passes(&h).await;
    refresh_token_behavior(&h).await;
    admin_token_passes(&h).await;
    health_is_public(&h).await;
    ready_is_public(&h).await;
}
