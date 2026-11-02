use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: String,
    pub user_id: String,
    //pub token_hash: String,
    pub u_role: String,
    pub family_id: String,
    pub rotated: bool,
    pub revoked: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub async fn insert_refresh_token(
    pool: &PgPool,
    user_id: &str,
    token_hash: &str,
    family_id: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let user_id = Uuid::parse_str(user_id)?;
    let family_id = Uuid::parse_str(family_id)?;

    sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id , user_id, token_hash, u_role, family_id, expires_at , created_at)
        VALUES ($1, $2, $3, $4, $5 , $6, $7)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(token_hash)
    .bind("USER") // default role for refresh tokens
    .bind(family_id)
    .bind(expires_at)
    .bind(chrono::Utc::now())
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
    let id = Uuid::parse_str(id)?;

    sqlx::query("UPDATE refresh_tokens SET rotated = TRUE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn revoke_family(pool: &PgPool, family_id: &str) -> Result<(), AppError> {
    let family_id = Uuid::parse_str(family_id)?;
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
    let user_id = Uuid::parse_str(user_id)?;
    sqlx::query("DELETE FROM refresh_tokens WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
