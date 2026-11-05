use crate::common::*;

use chrono::{Duration, Utc};
use order_service::order::repo::{OrderRepo, OrderRow};
use order_service::outbox::publisher;
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;

fn make_order(key: &str) -> OrderRow {
    let now = Utc::now();
    OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: key.into(),
        status: "pending".into(),
        total_amount: BigDecimal::from_str("1.00").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(600),
        created_at: now,
        updated_at: now,
    }
}

pub async fn publish_batch_marks_events_published(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

    let row = make_order("outbox-1");
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    OrderRepo::transition(
        &state.pool,
        &row.id,
        &["pending"],
        "reserved",
        Some("res-x"),
        None,
        None,
        "order.reserved",
        serde_json::json!({"reservation_id": "res-x"}),
    )
    .await
    .unwrap();

    let pending: (i64,) =
        sqlx::query_as("SELECT count(*) FROM outbox_events WHERE published_at IS NULL")
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(pending.0, 1);

    let n = publisher::publish_batch(&state.pool).await.unwrap();
    assert_eq!(n, 1);

    let left: (i64,) =
        sqlx::query_as("SELECT count(*) FROM outbox_events WHERE published_at IS NULL")
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(left.0, 0);

    let n2 = publisher::publish_batch(&state.pool).await.unwrap();
    assert_eq!(n2, 0);

    cleanup(&state.pool).await;
}

pub async fn publish_batch_empty_returns_zero(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

    let n = publisher::publish_batch(&state.pool).await.unwrap();
    assert_eq!(n, 0);

    cleanup(&state.pool).await;
}
