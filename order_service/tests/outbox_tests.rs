mod common;

use chrono::{Duration, Utc};
use order_service::order::repo::{OrderRepo, OrderRow};
use order_service::outbox::publisher;
use sqlx::types::BigDecimal;
use std::str::FromStr;

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

#[tokio::test]
async fn publish_batch_marks_events_published() {
    let state = common::setup_pool().await;
    common::cleanup(&state.0.clone()).await;
    let state = common::setup_state(state.0, state.1, state.2).await;

    let row = make_order("outbox-1");
    OrderRepo::insert(&state.db, &row, &[]).await.unwrap();

    OrderRepo::transition(
        &state.db,
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
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(pending.0, 1);

    let n = publisher::publish_batch(&state.db).await.unwrap();
    assert_eq!(n, 1);

    let left: (i64,) =
        sqlx::query_as("SELECT count(*) FROM outbox_events WHERE published_at IS NULL")
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(left.0, 0);

    let n2 = publisher::publish_batch(&state.db).await.unwrap();
    assert_eq!(n2, 0);

    common::cleanup(&state.db).await;
}

#[tokio::test]
async fn publish_batch_empty_returns_zero() {
    let state = common::setup_pool().await;
    common::cleanup(&state.0.clone()).await;
    let state = common::setup_state(state.0, state.1, state.2).await;

    let n = publisher::publish_batch(&state.db).await.unwrap();
    assert_eq!(n, 0);

    common::cleanup(&state.db).await;
}
