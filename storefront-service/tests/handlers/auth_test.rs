use std::sync::Arc;

use crate::common::{TestHarness, USER_ID, auth, get_json};
pub async fn missing_authorization_returns_401(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json(&h.app, "/api/v1/orders", &[]).await;
    assert_eq!(status, 401);
}

pub async fn non_bearer_scheme_returns_401(h: &Arc<TestHarness>) {
    let token = auth::access_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &format!("Basic {token}"))],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn malformed_jwt_returns_401(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", "Bearer not.a.jwt")],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn jwt_signed_with_wrong_secret_returns_401(h: &Arc<TestHarness>) {
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    use storefront_service::middleware::auth::Claims;

    let claims = Claims {
        aud: "".to_string(),
        exp: 8787,
        iat: 99,
        iss: "".to_string(),
        jti: "".to_string(),
        role: "USER".to_string(),
        sub: USER_ID.to_string(),
        typ: "access".to_string(),
    };
    let bad_token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(b"totally-wrong-secret"),
    )
    .unwrap();

    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &format!("Bearer {bad_token}"))],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn expired_jwt_returns_401(h: &Arc<TestHarness>) {
    let token = auth::expired_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 401);
}

pub async fn valid_access_token_passes(h: &Arc<TestHarness>) {
    let token = auth::access_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 200);
}

pub async fn refresh_token_behavior(h: &Arc<TestHarness>) {
    let token = auth::refresh_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;

    assert_eq!(status, 200);
}

pub async fn admin_token_passes(h: &Arc<TestHarness>) {
    let token = auth::admin_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 200);
}

pub async fn health_is_public(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json(&h.app, "/health", &[]).await;
    assert_eq!(status, 200);
}

pub async fn ready_is_public(h: &Arc<TestHarness>) {
    let (status, _, _) = get_json(&h.app, "/ready", &[]).await;
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
