use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// Returns a pool connection to the test DB and it runs migrations
/// and truncates catalog tables so each test starts clean..
///
/// Requires env: TEST_DATABASE_URL (falls back to localhost)...
pub async fn setup() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/catalog_test".into());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("connect test db (set TEST_DATABASE_URL)");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    sqlx::query(
        "TRUNCATE product_variants, products, categories RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");

    pool
}