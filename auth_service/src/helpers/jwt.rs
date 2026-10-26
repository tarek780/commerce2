use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    pub typ: String,
    pub role: String,
}

pub fn issue_token(
    user_id: String,
    role: &str,
    typ: &str,
    expires_in_secs: i64,
) -> Result<String, AppError> {
    let now = Utc::now();
    let secret = std::env::var("SECRET_KEY")?;
    let issuer = std::env::var("JWT_ISSUER")?;
    let audience = std::env::var("JWT_AUDIENCE ")?;
    let claims = Claims {
        sub: user_id.into(),
        iss: issuer.into(),
        aud: audience.into(),
        exp: (now + Duration::seconds(expires_in_secs)).timestamp() as usize,
        iat: now.timestamp() as usize,
        jti: Uuid::new_v4().into(),
        typ: typ.into(),
        role: role.into(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
}

pub fn verify_token(token: &str) -> Result<Claims, AppError> {
    let secret: String = std::env::var("SECRET_KEY")?;
    let issuer = std::env::var("JWT_ISSUER")?;
    let audience = std::env::var("JWT_AUDIENCE ")?;
    let mut v = Validation::new(Algorithm::HS256);
    v.set_issuer(&[issuer]);
    v.set_audience(&[audience]);
    v.validate_exp = true;

    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &v)
        .map_err(|_| AppError::Unauthorized)?;

    Ok(data.claims)
}
