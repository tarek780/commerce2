use order_service::state::AppState;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;
use testcontainers::{
    core::{ImageExt, WaitFor},
    runners::AsyncRunner,
};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;
pub struct TestAppState {
    pub pool: PgPool,
    pub appstate: Arc<AppState>,
    pub database_url: String,
    pub kafka_brokers: String,
    pub pg_container: ContainerAsync<GenericImage>,
    pub kafka_container: ContainerAsync<GenericImage>,
}

pub async fn setup_test_state() -> Arc<TestAppState> {
    dotenvy::dotenv().ok();
    let pg_image = GenericImage::new("postgres", "16.15-alpine3.24")
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_USER", "myuser")
        .with_env_var("POSTGRES_PASSWORD", "mypassword")
        .with_env_var("POSTGRES_DB", "mydb");

    let pg_container: ContainerAsync<GenericImage> = pg_image
        .start()
        .await
        .expect("Failed to start Postgres container");

    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

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
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(2))
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
                        format!(
                            "[ step {}/{}] the error : 500 : {}\n",
                            attempts, max_attempts, e
                        )
                        .as_bytes(),
                    )
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }
    }

    let pool = pool_option.unwrap_or_else(|| {
        panic!(
            "Failed to connect to Postgres after {} attempts",
            max_attempts
        );
    });

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    // apache/kafka:latest
    let kafka_image = GenericImage::new("apache/kafka", "latest")
        .with_wait_for(WaitFor::message_on_stdout("Kafka startTimeMs"))
        .with_env_var("KAFKA_BROKER_ID", "1")
        .with_env_var("KAFKA_ZOOKEEPER_CONNECT", "zookeeper:2181")
        .with_env_var("KAFKA_ADVERTISED_LISTENERS", "PLAINTEXT://localhost:9092")
        .with_env_var("KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR", "1");

    let kafka_container = kafka_image
        .start()
        .await
        .expect("Failed to start Kafka container");

    let kafka_brokers = format!(
        "localhost:{}",
        kafka_container
            .get_host_port_ipv4(9092)
            .await
            .expect("Failed to get Kafka container port")
    );
    let appstate = setup_state(pool.clone(), database_url.clone(), kafka_brokers.clone()).await;

    Arc::new(TestAppState {
        pool,
        database_url,
        kafka_brokers,
        appstate,
        kafka_container,
        pg_container,
    })
}

pub async fn cleanup(pool: &PgPool) {
    sqlx::query("TRUNCATE outbox_events, order_items, orders RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate");
}

pub async fn setup_state(pool: PgPool, db_url: String, kafka: String) -> Arc<AppState> {
    use order_service::config::OrderConfig;
    let cfg = OrderConfig {
        database_url: db_url,
        bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:0".into()),
        inventory_grpc_url: std::env::var("INVENTORY_GRPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1".into()),
        payment_grpc_url: std::env::var("PAYMENT_GRPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:1".into()),
        kafka_brokers: kafka,
        kafka_payment_topic: "payment.events".into(),
        kafka_payment_group: "test-group".into(),
        order_ttl_secs: 900,
        expiry_scan_secs: 30,
    };
    use order_service::clients::inventory::InventoryClient;
    use order_service::clients::payment::PaymentClient;
    use tonic::transport::Endpoint;
    let inv_channel = Endpoint::from_shared(cfg.inventory_grpc_url.clone())
        .unwrap()
        .connect_lazy();
    let pay_channel = Endpoint::from_shared(cfg.payment_grpc_url.clone())
        .unwrap()
        .connect_lazy();
    Arc::new(AppState {
        db: pool,
        inventory: Arc::new(InventoryClient::new(inv_channel)),
        payment: Arc::new(PaymentClient::new(pay_channel)),
        config: Arc::new(cfg),
    })
}
