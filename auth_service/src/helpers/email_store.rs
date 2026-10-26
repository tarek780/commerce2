use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{error::AppError, helpers::token};

const PREFIX: &str = "email_verify:";
const RATE_PREFIX: &str = "email_verify_rl:";

pub async fn issue_token(
    redis: &ConnectionManager,
    email: &str,

    expiry_secs: u64,
    rate_limit_secs: u64,
) -> Result<String, AppError> {
    let mut conn = redis.clone();
    //

    // rate limit
    let rl_key = format!("{RATE_PREFIX}{email}");
    //
    let exists: bool = conn
        .exists(&rl_key)
        .await
        .map_err(|e| AppError::Internal(format!("redis exists: {e}")))?;
    if exists {
        return Err(AppError::Conflict(
            "verification email recently sent".into(),
        ));
    }

    let token = token::generate_opaque_token();
    let key = format!("{PREFIX}{token}");

    let _: () = conn
        .set_ex(&key, email, expiry_secs)
        .await
        .map_err(|e| AppError::Internal(format!("redis set_ex: {e}")))?;

    //   limit
    let _: () = conn
        .set_ex(&rl_key, "1", rate_limit_secs)
        .await
        .map_err(|e| AppError::Internal(format!("redis set rl: {e}")))?;

    tracing::debug!(email = %email, "issued email verification token");
    Ok(token)
}

pub async fn consume_token(redis: &ConnectionManager, token: &str) -> Result<String, AppError> {
    let mut conn = redis.clone();
    let key = format!("{PREFIX}{token}");

    let email: Option<String> = conn
        .get(&key)
        .await
        .map_err(|e| AppError::Internal(format!("redis get: {e}")))?;

    let email = email.ok_or(AppError::Validation("invalid or expired token".into()))?;

    let _: () = conn
        .del(&key)
        .await
        .map_err(|e| AppError::Internal(format!("redis del: {e}")))?;

    tracing::info!(%email, "email verification token consumed");
    Ok(email)
}
