mod common;

use order_service::clients::inventory::InventoryClient;
use order_service::clients::payment::PaymentClient;
use order_service::order::orchestrator::{self, NewItem};
use order_service::state::AppState;
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::transport::Endpoint;

fn new_item(v: i32, q: i32, price: &str) -> NewItem {
    NewItem {
        variant_id: v,
        quantity: q,
        unit_price: BigDecimal::from_str(price).unwrap(),
    }
}

/// يبني AppState بنفس الـ pool لكن مع gRPC clients معطّلة.
/// تُستخدم فقط في اختبارات لا تصل إلى inventory/payment.
async fn state_with_pool(pool: sqlx::PgPool) -> Arc<AppState> {
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
    let inv = Endpoint::from_shared(cfg.inventory_grpc_url.clone())
        .unwrap()
        .connect_lazy();
    let pay = Endpoint::from_shared(cfg.payment_grpc_url.clone())
        .unwrap()
        .connect_lazy();

    Arc::new(AppState {
        db: pool,
        inventory: Arc::new(InventoryClient::new(inv)),
        payment: Arc::new(PaymentClient::new(pay)),
        config: Arc::new(cfg),
    })
}

#[tokio::test]
async fn empty_items_rejected() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;
    let state = state_with_pool(pool.clone()).await;

    let err = orchestrator::create_and_orchestrate(
        &state,
        "cust",
        "USD",
        vec![],
        "idem-1",
    )
    .await
    .unwrap_err();

    assert!(matches!(err, order_service::error::OrderError::InvalidState(_)));
    common::cleanup(&pool).await;
}

#[tokio::test]
async fn cancel_final_state_rejected() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;
    let state = state_with_pool(pool.clone()).await;

    use order_service::order::repo::{OrderRepo, OrderRow};
    use chrono::{Duration, Utc};
    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: "k-cancel-final".into(),
        status: "confirmed".into(),
        total_amount: BigDecimal::from_str("1.00").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(60),
        created_at: now,
        updated_at: now,
    };
    OrderRepo::insert(&pool, &row, &[]).await.unwrap();

    let err = orchestrator::cancel(&state, &row.id, "test").await.unwrap_err();
    assert!(matches!(err, order_service::error::OrderError::InvalidState(_)));

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn cancel_pending_succeeds_without_clients() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;
    let state = state_with_pool(pool.clone()).await;

    use order_service::order::repo::{OrderRepo, OrderRow};
    use chrono::{Duration, Utc};
    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: "k-cancel-pending".into(),
        status: "pending".into(),
        total_amount: BigDecimal::from_str("1.00").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(60),
        created_at: now,
        updated_at: now,
    };
    OrderRepo::insert(&pool, &row, &[]).await.unwrap();

    let out = orchestrator::cancel(&state, &row.id, "user_cancel").await.unwrap();
    assert_eq!(out.status, "cancelled");
    assert_eq!(out.failure_reason.as_deref(), Some("user_cancel"));

    common::cleanup(&pool).await;
}



#[tokio::test]
#[ignore = "requires running inventory & payment services"]
async fn happy_path_full_orchestration() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;

    let cfg = order_service::config::OrderConfig {
        database_url: std::env::var("TEST_DATABASE_URL").unwrap(),
        bind_addr: "0.0.0.0:0".into(),
        inventory_grpc_url: std::env::var("TEST_INVENTORY_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:50051".into()),
        payment_grpc_url: std::env::var("TEST_PAYMENT_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:50052".into()),
        kafka_brokers: "127.0.0.1:9092".into(),
        kafka_payment_topic: "payment.events".into(),
        kafka_payment_group: "test-group".into(),
        order_ttl_secs: 900,
        expiry_scan_secs: 30,
    };
    let state = Arc::new(AppState::new(cfg).await.unwrap());

    let items = vec![new_item(1, 1, "9.99")];
    let row = orchestrator::create_and_orchestrate(&state, "c1", "USD", items, "idem-happy")
        .await
        .unwrap();

    assert_eq!(row.status, "awaiting_payment");
    assert!(row.reservation_id.is_some());
    assert!(row.payment_id.is_some());

    common::cleanup(&pool).await;
}