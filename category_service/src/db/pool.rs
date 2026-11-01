
use std::time::Duration;
use sqlx::{postgres::PgPoolOptions, PgPool};
use crate::error::AppError;

pub async fn create_pool(url: &str) -> Result<PgPool, AppError> {
    Ok(PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(url).await?)
}