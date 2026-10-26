use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, models::user::User};

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    display_name: Option<&str>,
    full_name: &str,
    username: &str,
    u_role: &str,
) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, display_name , full_name , username , u_role )
        VALUES ($1, $2, $3 , $4 , $5 , $6)
        RETURNING *
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(full_name)
    .bind(username)
    .bind(u_role)
    .fetch_one(pool)
    .await
    .map_err(map_unique_email)?;

    Ok(user)
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn find_by_id(pool: &PgPool, id: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn mark_email_verified(pool: &PgPool, email: &str) -> Result<u64, AppError> {
    let res = sqlx::query(
        "UPDATE users SET email_verified_at = NOW(), updated_at = NOW() WHERE email = $1",
    )
    .bind(email)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

pub async fn update_display_name(
    pool: &PgPool,
    id: &str,
    display_name: Option<&str>,
) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET display_name = COALESCE($2, display_name),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(display_name)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn delete(pool: &PgPool, user_id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

fn map_unique_email(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db) = &e {
        if db.constraint() == Some("users_email_key") {
            return AppError::Conflict("email already registered".into());
        }
    }
    AppError::Database(e)
}

#[allow(dead_code)]
pub fn now_utc() -> chrono::DateTime<Utc> {
    Utc::now()
}
