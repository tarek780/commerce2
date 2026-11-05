#![allow(dead_code)]

use payment_service::config::PaymentConfig;
use payment_service::gateway;
use payment_service::state::AppState;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;

use testcontainers::{
    GenericImage,
    core::{ImageExt, WaitFor},
    runners::AsyncRunner,
};

pub fn test_config(db_url: String, kafka_broker: String) -> PaymentConfig {
    PaymentConfig {
        database_url: db_url,
        grpc_addr: "127.0.0.1:6001".into(),
        http_addr: "127.0.0.1:6000".into(),
        gateway: "mock".into(),
        gateway_public_base: "http://localhost/pay".into(),
        webhook_secret: "test-secret".into(),
        reservation_ttl_secs: 900,
        expiry_scan_secs: 10,
        kafka_brokers: kafka_broker,
        kafka_payment_topic: "payment.events.test".into(),
    }
}

pub async fn test_state() -> Arc<AppState> {
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

    let pg_port = pg_container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get Postgres container port");

    let database_url =
        format!("postgres://myuser:mypassword@127.0.0.1:{pg_port}/mydb?sslmode=disable");

    let kafka_image = GenericImage::new("apache/kafka", "latest")
        .with_wait_for(WaitFor::message_on_stdout("Kafka Server started"))
        .with_env_var("KAFKA_NODE_ID", "1")
        .with_env_var("KAFKA_PROCESS_ROLES", "broker,controller")
        .with_env_var(
            "KAFKA_LISTENERS",
            "PLAINTEXT://0.0.0.0:9092,CONTROLLER://0.0.0.0:9093",
        )
        .with_env_var("KAFKA_ADVERTISED_LISTENERS", "PLAINTEXT://127.0.0.1:9092")
        .with_env_var("KAFKA_CONTROLLER_LISTENER_NAMES", "CONTROLLER")
        .with_env_var(
            "KAFKA_LISTENER_SECURITY_PROTOCOL_MAP",
            "PLAINTEXT:PLAINTEXT,CONTROLLER:PLAINTEXT",
        )
        .with_env_var("KAFKA_CONTROLLER_QUORUM_VOTERS", "1@127.0.0.1:9093")
        .with_env_var("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "1");

    let kafka_container = kafka_image
        .start()
        .await
        .expect("Failed to start Kafka container");

    let kafka_port = kafka_container
        .get_host_port_ipv4(9092)
        .await
        .expect("Failed to get Kafka container port");

    let kafka_broker = format!("127.0.0.1:{kafka_port}");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect test postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrations failed");

    sqlx::query("TRUNCATE payments, payment_attempts, outbox_events, processed_webhooks RESTART IDENTITY CASCADE")
        .execute(&pool)
        .await
        .expect("truncate failed");

    let cfg = test_config(database_url, kafka_broker.clone());

    let _gateway_client = gateway::build(&cfg);

    //
    let kafka_topic =
        std::env::var("KAFKA_PAYMENT_TOPIC").unwrap_or_else(|_| "payment.events".to_string());
    let kafka = match payment_service::payment::kafka_publisher::KafkaPublisher::new(
        kafka_broker.as_str(),
        kafka_topic.as_str(),
    ) {
        Ok(k) => {
            tracing::info!("kafka publisher ready");
            Some(Arc::new(k))
        }
        Err(e) => {
            tracing::warn!(error = %e, "kafka publisher unavailable; using outbox only");
            None
        }
    };
    Arc::new(AppState {
        gateway: _gateway_client,
        kafka: kafka,
        db: pool,
        config: Arc::new(cfg),
    })
}

pub async fn setup_pool() -> PgPool {
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

    let pg_port = pg_container
        .get_host_port_ipv4(5432)
        .await
        .expect("Failed to get Postgres container port");

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
