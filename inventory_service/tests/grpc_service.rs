mod common;

use std::sync::Arc;
use std::time::Duration;

use inventory_service::inventory::repo::InventoryRepo;
use inventory_service::inventory::service::InventorySvc;
use inventory_service::pb::inventory_service_client::InventoryServiceClient;
use inventory_service::pb::inventory_service_server::InventoryServiceServer;
use inventory_service::pb::*;

use tokio::sync::broadcast;
use tonic::transport::{Channel, Server};
use uuid::Uuid;

async fn spawn_server(env: &common::TestEnv) -> (Channel, tokio::task::JoinHandle<()>) {
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

    let mut channel = None;
    for _ in 0..50 {
        if let Ok(ch) = Channel::from_shared(format!("http://{}", addr))
            .unwrap()
            .connect()
            .await
        {
            channel = Some(ch);
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    (channel.expect("connect to test server"), handle)
}

#[tokio::test]
async fn grpc_reserve_and_get_stock() {
    let env = require_db!();
    let vid = Uuid::now_v7();
    common::seed_inventory(&env.pool, vid, 5).await;

    let (channel, handle) = spawn_server(&env).await;
    let mut client = InventoryServiceClient::new(channel);

    let resp = client
        .reserve(ReserveRequest {
            reservation_key: common::new_reservation_key(),
            variant_id: vid.to_string(),
            quantity: 2,
            ttl_seconds: 60,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.variant_id, vid.to_string());
    assert_eq!(resp.quantity, 2);
    assert!(resp.expires_at_unix > chrono::Utc::now().timestamp());

    let stock = client
        .get_stock(GetStockRequest {
            variant_id: vid.to_string(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(stock.stock_quantity, 5);
    assert_eq!(stock.reserved_quantity, 2);
    assert_eq!(stock.available_quantity, 3);

    handle.abort();
    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn grpc_insufficient_stock_maps_to_failed_precondition() {
    let env = require_db!();
    let vid = Uuid::now_v7();
    common::seed_inventory(&env.pool, vid, 1).await;

    let (channel, handle) = spawn_server(&env).await;
    let mut client = InventoryServiceClient::new(channel);

    let err = client
        .reserve(ReserveRequest {
            reservation_key: common::new_reservation_key(),
            variant_id: vid.to_string(),
            quantity: 10,
            ttl_seconds: 60,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
    assert!(err.message().contains("insufficient"));

    handle.abort();
    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn grpc_unknown_variant_maps_to_not_found() {
    let env = require_db!();
    let vid = Uuid::now_v7();

    let (channel, handle) = spawn_server(&env).await;
    let mut client = InventoryServiceClient::new(channel);

    let err = client
        .get_stock(GetStockRequest {
            variant_id: vid.to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);

    handle.abort();
}

#[tokio::test]
async fn grpc_commit_and_adjust() {
    let env = require_db!();
    let vid = Uuid::now_v7();
    common::seed_inventory(&env.pool, vid, 10).await;

    let (channel, handle) = spawn_server(&env).await;
    let mut client = InventoryServiceClient::new(channel);

    let res = client
        .reserve(ReserveRequest {
            reservation_key: common::new_reservation_key(),
            variant_id: vid.to_string(),
            quantity: 3,
            ttl_seconds: 60,
        })
        .await
        .unwrap()
        .into_inner();

    client
        .commit(CommitRequest {
            reservation_id: res.reservation_id.clone(),
        })
        .await
        .unwrap();

    let stock = client
        .get_stock(GetStockRequest {
            variant_id: vid.to_string(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(stock.stock_quantity, 7);
    assert_eq!(stock.reserved_quantity, 0);

    let adj = client
        .adjust_stock(AdjustStockRequest {
            variant_id: vid.to_string(),
            delta: 5,
            reference_id: "po-123".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(adj.new_stock_quantity, 12);

    handle.abort();
    common::cleanup_variant(&env.pool, vid).await;
}
