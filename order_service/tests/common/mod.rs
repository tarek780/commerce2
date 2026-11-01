use order_service::state::AppState;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::sync::Arc;
use testcontainers::{
    ContainerAsync,
    core::{Image, ImageExt, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use testcontainers::{ContainerRequest, GenericImage};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;


pub async fn setup_pool() -> PgPool {

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


pub async fn cleanup(pool: &PgPool) {
    sqlx::query("TRUNCATE outbox_events, order_items, orders RESTART IDENTITY CASCADE")
        .execute(pool)
        .await
        .expect("truncate");
}



pub async fn setup_state(pool: PgPool) -> Arc<AppState> {
    use order_service::config::OrderConfig;

    let cfg = OrderConfig {
        database_url: std::env::var("TEST_DATABASE_URL").unwrap(),
        bind_addr: "0.0.0.0:0".into(),
        inventory_grpc_url: "http://127.0.0.1:1".into(),
        payment_grpc_url: "http://127.0.0.1:1".into(),
        kafka_brokers: "127.0.0.1:1".into(),
        kafka_payment_topic: "payment.events".into(),
        kafka_payment_group: "test-group".into(),
        order_ttl_secs: 900,
        expiry_scan_secs: 30,
    };

    // نتحايل: نبني clients بدون اتصال فعلي
    use order_service::clients::inventory::InventoryClient;
    use order_service::clients::payment::PaymentClient;
    use tonic::transport::{Channel, Endpoint};

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