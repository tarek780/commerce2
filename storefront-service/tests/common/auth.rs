use std::sync::Once;
use storefront_service::middleware::auth::Claims;

static INIT: Once = Once::new();

pub fn init_secret_once() {
    INIT.call_once(|| unsafe {
        std::env::set_var("SECRET_KEY", "test-secret-key-for-integration-tests-only");
    });
}

pub fn secret() -> &'static str {
    "test-secret-key-for-integration-tests-only"
}

pub fn access_token_for(user_id: &str) -> String {
    token_with(user_id, "Access", "User", 3600)
}

pub fn refresh_token_for(user_id: &str) -> String {
    token_with(user_id, "Refresh", "User", 3600)
}

pub fn expired_token_for(user_id: &str) -> String {
    token_with(user_id, "Access", "User", -10)
}

pub fn admin_token_for(user_id: &str) -> String {
    token_with(user_id, "Access", "Admin", 3600)
}

fn token_with(user_id: &str, typ: &str, role: &str, exp_offset_secs: i64) -> String {
    use chrono::Utc;
    use jsonwebtoken::{Algorithm, EncodingKey, Header};

    let now = Utc::now().timestamp();
    let exp = now + exp_offset_secs;

    let claims = Claims {
        sub: user_id.to_string(),
        iss: "test-issuer".to_string(),
        aud: "test-audience".to_string(),
        exp: exp as usize,
        iat: now as usize,
        jti: uuid::Uuid::new_v4().to_string(), // تحتاج مكتبة uuid لتوليد معرف فريد، أو استبدلها بنص فارغ ""
        typ: typ.to_string(),
        role: role.to_string(),
    };

    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret().as_bytes()),
    )
    .expect("encode jwt")
}

pub fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}
