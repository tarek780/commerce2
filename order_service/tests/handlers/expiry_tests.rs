use crate::common::*;
use chrono::{Duration, Utc};
use order_service::order::expiry;
use order_service::order::repo::{OrderRepo, OrderRow};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;

pub async fn expire_batch_marks_due_orders_cancelled(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

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
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    let n: u64 = expiry::expire_batch(&state.appstate).await.unwrap();
    assert_eq!(n, 1);

    let after = OrderRepo::get(&state.pool, &row.id).await.unwrap();
    assert_eq!(after.status, "cancelled");
    assert_eq!(after.failure_reason.as_deref(), Some("expired_ttl"));

    cleanup(&state.pool.clone()).await;
}

pub async fn expire_batch_skips_not_due(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

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
    OrderRepo::insert(&state.pool.clone(), &row, &[])
        .await
        .unwrap();

    let n = expiry::expire_batch(&state.appstate.clone()).await.unwrap();
    assert_eq!(n, 0);

    cleanup(&state.pool.clone()).await;
}
