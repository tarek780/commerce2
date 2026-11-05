use redis::Client;
use redis::aio::{ConnectionManager, MultiplexedConnection};

use testcontainers::{ContainerAsync, GenericImage};
use testcontainers::{core::WaitFor, runners::AsyncRunner};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;

pub async fn setup() -> (
    Client,
    ConnectionManager,
    MultiplexedConnection,
    ContainerAsync<GenericImage>,
) {
    dotenvy::dotenv().ok();

    let redis_image = GenericImage::new("redis", "alpine3.23")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

    let redis_container = redis_image
        .start()
        .await
        .expect("Failed to start Redis container");

    let redis_host = redis_container.get_host().await.unwrap();
    let redis_port = match redis_container.get_host_port_ipv4(6379).await {
        Ok(c) => c,
        Err(e) => {
            let _ = stdout()
                .write_all(format!("Failed to get Redis container port: {}\n", e).as_bytes())
                .await;
            panic!("Failed to get Redis container port: {}", e);
        }
    };
    let redis_url = format!("redis://{redis_host}:{redis_port}");

    let redis_client = redis::Client::open(redis_url).expect("redis client");
    let redis = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("redis connection");
    let redis_manager = ConnectionManager::new(redis_client.clone()).await.unwrap();
    (redis_client, redis_manager, redis, redis_container)
    //
}

/// Helper to build unique keys for  tests...
pub fn uniq(prefix: &str) -> String {
    format!("test:{prefix}:{}", uuid::Uuid::now_v7())
}
