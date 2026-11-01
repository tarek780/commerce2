use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;
   use testcontainers::{
    ContainerAsync,
    core::{Image, ImageExt, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use testcontainers::{ContainerRequest, GenericImage};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;

/// Returns a pool connection to the test DB and it runs migrations
/// and truncates catalog tables so each test starts clean..
///
/// Requires env: TEST_DATABASE_URL (falls back to localhost)...
pub async fn setup() -> PgPool {


      
        unsafe {
            std::env::set_var("SECRET_KEY", "test-secret-key-32-chars-minimum-xx");
            std::env::set_var("RUST_LOG", "info");
            std::env::set_var("RUST_BACKTRACE", "1");
        }

        let pg_image = GenericImage::new("postgres", "16.15-alpine3.24")
            .with_wait_for(WaitFor::message_on_stdout(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_USER", "myuser")
            .with_env_var("POSTGRES_PASSWORD", "mypassword")
            .with_env_var("POSTGRES_DB", "mydb");

        let pg_container = pg_image
            .start()
            .await
            .expect("Failed to start Postgres container");

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let pg_port = match pg_container.get_host_port_ipv4(5432).await {
            Ok(c) => c,
            Err(e) => {
                let _ = stdout()
                    .write_all(format!("Failed to get Postgres container port: {}\n", e).as_bytes())
                    .await;
                panic!("Failed to get Postgres container port: {}", e);
            }
        };

        let database_url =
            format!("postgres://myuser:mypassword@127.0.0.1:{pg_port}/mydb?sslmode=disable");

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await
            .expect("connect test postgres");

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