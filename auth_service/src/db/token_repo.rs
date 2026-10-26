use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub u_role: String,
    pub family_id: String,
    pub rotated: bool,
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
}

pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: String,
    token_hash: &str,
    family_id: String,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (user_id, token_hash, family_id, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(family_id)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_hash(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshTokenRow>, AppError> {
    let row =
        sqlx::query_as::<_, RefreshTokenRow>("SELECT * FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .fetch_optional(pool)
            .await?;
    Ok(row)
}

pub async fn mark_rotated(pool: &PgPool, id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE refresh_tokens SET rotated = TRUE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_family(pool: &PgPool, family_id: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE family_id = $1")
        .bind(family_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_by_hash(pool: &PgPool, token_hash: &str) -> Result<(), AppError> {
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &PgPool, user_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
