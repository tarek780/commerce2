use crate::common::*;
use inventory_service::inventory::repo::InventoryRepo;
use inventory_service::inventory::service::InventorySvc;
use inventory_service::pb::inventory_service_client::InventoryServiceClient;
use inventory_service::pb::inventory_service_server::InventoryServiceServer;
use inventory_service::pb::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tonic::transport::{Channel, Server};
use uuid::Uuid;

async fn spawn(env: Arc<TestEnv>) -> (Channel, tokio::task::JoinHandle<()>) {
    let (tx, _) = broadcast::channel(1024);
    let repo = Arc::new(InventoryRepo::new(env.pool.clone()));
    let svc = InventorySvc::new(repo, tx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = Server::builder()
        .add_service(InventoryServiceServer::new(svc))
        .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener));
    let handle = tokio::spawn(async move {
        let _ = server.await;
    });

    let channel = Channel::from_shared(format!("http://{}", addr))
        .unwrap()
        .connect()
        .await
        .expect("connect");
    (channel, handle)
}

pub async fn watch_receives_reserved_event(env: Arc<TestEnv>) {
    let vid = Uuid::now_v7();
    seed_inventory(&env.pool, vid, 5).await;

    let (channel, handle) = spawn(env.clone()).await;
    let mut client = InventoryServiceClient::new(channel);

    let mut stream = client
        .watch_stock(WatchStockRequest {
            variant_ids: vec![vid.to_string()],
            low_stock_threshold: 100,
        })
        .await
        .unwrap()
        .into_inner();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let _ = match client
        .reserve(ReserveRequest {
            reservation_key: new_reservation_key(),
            variant_id: vid.to_string(),
            quantity: 2,
            ttl_seconds: 60,
        })
        .await
    {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };

    let ev = tokio::time::timeout(Duration::from_secs(2), stream.message())
        .await
        .expect("timeout waiting for stream event")
        .expect("stream error")
        .expect("stream closed");

    assert_eq!(ev.variant_id, vid.to_string());
    assert_eq!(ev.event_type, "reserved");
    assert_eq!(ev.reserved_quantity, 2);

    handle.abort();
    cleanup_variant(&env.pool, vid).await;
}
