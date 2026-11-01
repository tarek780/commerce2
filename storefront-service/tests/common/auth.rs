use std::sync::Once;
use storefront_service::middleware::auth::{DecodedToken, Role, Typ};

static INIT: Once = Once::new();

///  SECRET_KEY 
pub fn init_secret_once() {
    INIT.call_once(|| {
        std::env::set_var("SECRET_KEY", "test-secret-key-for-integration-tests-only");
    });
}

pub fn secret() -> &'static str {
    "test-secret-key-for-integration-tests-only"
}

///  access token gen
pub fn access_token_for(user_id: &str) -> String {
    token_with(user_id, Typ::Access, Role::User, 3600)
}

/// refresh token expired
pub fn refresh_token_for(user_id: &str) -> String {
    token_with(user_id, Typ::Refresh, Role::User, 3600)
}

/// access token  expired
pub fn expired_token_for(user_id: &str) -> String {
    token_with(user_id, Typ::Access, Role::User, -10)
}

pub fn admin_token_for(user_id: &str) -> String {
    token_with(user_id, Typ::Access, Role::Admin, 3600)
}

fn token_with(user_id: &str, typ: Typ, role: Role, exp_offset_secs: i64) -> String {
    use jsonwebtoken::{EncodingKey, Header, Algorithm};
    use chrono::Utc;

    let claims = DecodedToken {
        typ,
        user_id: user_id.to_string(),
        token_id: uuid::Uuid::new_v4().to_string(),
        role,
        exp: Utc::now().timestamp() + exp_offset_secs,
    };

    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret().as_bytes()),
    )
    .expect("encode jwt")
}

/// header
pub fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}