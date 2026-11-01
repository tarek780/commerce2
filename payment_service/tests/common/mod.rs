
#![allow(dead_code)]

use std::sync::Arc;

use payment_service::config::PaymentConfig;
use payment_service::gateway;
use payment_service::state::AppState;

use testcontainers::{
    ContainerAsync,
    core::{Image, ImageExt, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use testcontainers::{ContainerRequest, GenericImage};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;
 

pub fn test_config() -> PaymentConfig {
    PaymentConfig {
        database_url: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/payment_test".into()),
        grpc_addr: "127.0.0.1:0".into(),
        http_addr: "127.0.0.1:0".into(),
        gateway: "mock".into(),
        gateway_public_base: "http://localhost/pay".into(),
        webhook_secret: "test-secret".into(),
        reservation_ttl_secs: 900,
        expiry_scan_secs: 10,
        kafka_brokers: "localhost:9092".into(),
        kafka_payment_topic: "payment.events.test".into(),
    }
}

/// Builds AppState + runs migrations against TEST_DATABASE_URL.
/// Also truncates tables so each test starts clean.
pub async fn test_state() -> Arc<AppState> {

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
        .expect("migrations failed");

    // Clean slate
    sqlx::query("TRUNCATE payments, payment_attempts, outbox_events, processed_webhooks RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("truncate failed");

    let _ = gateway::build(&cfg); // ensure gateway builds
    state
}

pub async fn pool(){

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

       pool
}