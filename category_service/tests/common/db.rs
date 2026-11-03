use redis::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Pool, Postgres};
use testcontainers::{ContainerAsync, GenericImage};
use testcontainers::{
    core::{ImageExt, WaitFor},
    runners::AsyncRunner,
};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;

/// Returns a pool connection to the test DB and it runs migrations
/// and truncates catalog tables so each test starts clean..
///
/// Requires env: TEST_DATABASE_URL (falls back to localhost)...

pub async fn setup() -> PgPool {
    dotenvy::dotenv().ok();
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

    pool
}

pub async fn build_state() -> (PgPool, ContainerAsync<GenericImage>) {
    //
    dotenvy::dotenv().ok();
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

    let mut pool_option: Option<PgPool> = None;
    let mut attempts = 0;
    let max_attempts = 7;

    while attempts < max_attempts {
        match PgPoolOptions::new()
            .max_connections(20)
            .acquire_timeout(std::time::Duration::from_secs(20))
            // err : DB: pool timed out while waiting for an open connection
            .connect(&database_url)
            .await
        {
            Ok(p) => {
                pool_option = Some(p);
                break;
            }
            Err(e) => {
                attempts += 1;
                let _ = stdout()
                    .write_all(
                        format!("⚠️ [step : {}/{}] {}\n", attempts, max_attempts, e).as_bytes(),
                    )
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }

    let pool = pool_option.unwrap_or_else(|| {
        panic!(
            "failed to connect to database after {} attempts",
            max_attempts
        );
    });

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    sqlx::query("TRUNCATE product_variants, products, categories RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("truncate");

    //

    //
    (pool, pg_container)
}
