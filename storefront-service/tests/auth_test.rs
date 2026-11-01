mod common;
use common::{auth, get_json, USER_ID};


#[tokio::test]
async fn missing_authorization_returns_401() {
    let h = common::harness().await;
    let (status, _, _) = get_json(&h.app, "/api/v1/orders", &[]).await;
    assert_eq!(status, 401);
}


#[tokio::test]
async fn non_bearer_scheme_returns_401() {
    let h = common::harness().await;
    let token = auth::access_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &format!("Basic {token}"))],
    )
    .await;
    assert_eq!(status, 401);
}


#[tokio::test]
async fn malformed_jwt_returns_401() {
    let h = common::harness().await;
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", "Bearer not.a.jwt")],
    )
    .await;
    assert_eq!(status, 401);
}

#[tokio::test]
async fn jwt_signed_with_wrong_secret_returns_401() {
    use jsonwebtoken::{EncodingKey, Header, Algorithm};
    use chrono::Utc;
    use storefront_service::middleware::auth::{DecodedToken, Role, Typ};

    let claims = DecodedToken {
        typ: Typ::Access,
        user_id: USER_ID.into(),
        token_id: uuid::Uuid::new_v4().to_string(),
        role: Role::User,
        exp: Utc::now().timestamp() + 3600,
    };
    let bad_token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(b"totally-wrong-secret"),
    )
    .unwrap();

    let h = common::harness().await;
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &format!("Bearer {bad_token}"))],
    )
    .await;
    assert_eq!(status, 401);
}


#[tokio::test]
async fn expired_jwt_returns_401() {
    let h = common::harness().await;
    let token = auth::expired_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 401);
}


#[tokio::test]
async fn valid_access_token_passes() {
    let h = common::harness().await;
    let token = auth::access_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 200);
}


#[tokio::test]
async fn refresh_token_behavior() {
    let h = common::harness().await;
    let token = auth::refresh_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;

    assert_eq!(status, 200);
}


#[tokio::test]
async fn admin_token_passes() {
    let h = common::harness().await;
    let token = auth::admin_token_for(USER_ID);
    let (status, _, _) = get_json(
        &h.app,
        "/api/v1/orders",
        &[("authorization", &auth::bearer(&token))],
    )
    .await;
    assert_eq!(status, 200);
}


#[tokio::test]
async fn health_is_public() {
    let h = common::harness().await;
    let (status, _, _) = get_json(&h.app, "/health", &[]).await;
    assert_eq!(status, 200);
}


#[tokio::test]
async fn ready_is_public() {
    let h = common::harness().await;
    let (status, _, _) = get_json(&h.app, "/ready", &[]).await;
    assert_eq!(status, 200);
}