mod common;

use chrono::{Duration, Utc};
use order_service::order::repo::{OrderRepo, OrderRow};
use order_service::order::expiry;
use sqlx::types::BigDecimal;
use std::str::FromStr;

#[tokio::test]
async fn expire_batch_marks_due_orders_cancelled() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;
    let state = common::setup_state(pool.clone()).await;

    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: "expiry-1".into(),
        status: "pending".into(),
        total_amount: BigDecimal::from_str("1.00").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now - Duration::seconds(30),
        created_at: now,
        updated_at: now,
    };
    OrderRepo::insert(&pool, &row, &[]).await.unwrap();

    let n = expiry::expire_batch(&state).await.unwrap();
    assert_eq!(n, 1);

    let after = OrderRepo::get(&pool, &row.id).await.unwrap();
    assert_eq!(after.status, "cancelled");
    assert_eq!(after.failure_reason.as_deref(), Some("expired_ttl"));

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn expire_batch_skips_not_due() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;
    let state = common::setup_state(pool.clone()).await;

    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: "expiry-2".into(),
        status: "pending".into(),
        total_amount: BigDecimal::from_str("1.00").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(3600),
        created_at: now,
        updated_at: now,
    };
    OrderRepo::insert(&pool, &row, &[]).await.unwrap();

    let n = expiry::expire_batch(&state).await.unwrap();
    assert_eq!(n, 0);

    common::cleanup(&pool).await;
}