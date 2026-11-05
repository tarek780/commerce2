use crate::common::*;

use chrono::{Duration, Utc};
use order_service::error::OrderError;
use order_service::order::repo::{OrderRepo, OrderRow};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

fn sample_order(key: &str) -> OrderRow {
    let now = Utc::now();
    OrderRow {
        id: Uuid::now_v7().to_string(),
        customer_id: "cust-1".into(),
        idempotency_key: key.into(),
        status: "pending".into(),
        total_amount: BigDecimal::from_str("10.50").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(900),
        created_at: now,
        updated_at: now,
    }
}

pub async fn insert_and_get_order(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let row = sample_order("key-1");
    let items = vec![(Uuid::now_v7(), 2, BigDecimal::from_str("5.25").unwrap())];

    OrderRepo::insert(&state.pool, &row, &items).await.unwrap();
    let got = OrderRepo::get(&state.pool, &row.id).await.unwrap();
    assert_eq!(got.id, row.id);
    assert_eq!(got.status, "pending");
    assert_eq!(got.total_amount, row.total_amount);

    let got_items = OrderRepo::items(&state.pool, &row.id).await.unwrap();
    assert_eq!(got_items.len(), 1);
    assert_eq!(got_items[0].variant_id, 101);
    assert_eq!(got_items[0].quantity, 2);

    cleanup(&state.pool).await;
}

pub async fn duplicate_idempotency_key_returns_duplicate(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let r1 = sample_order("dup-key");
    OrderRepo::insert(&state.pool, &r1, &[]).await.unwrap();

    let r2 = sample_order("dup-key");
    let err = OrderRepo::insert(&state.pool, &r2, &[]).await.unwrap_err();
    assert!(matches!(err, OrderError::Duplicate));

    cleanup(&state.pool).await;
}

pub async fn get_missing_returns_not_found(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let err = OrderRepo::get(&state.pool, "missing-id").await.unwrap_err();
    assert!(matches!(err, OrderError::NotFound));

    cleanup(&state.pool).await;
}

pub async fn transition_pending_to_reserved(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let row = sample_order("trans-key-1");
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    let updated = OrderRepo::transition(
        &state.pool,
        &row.id,
        &["pending"],
        "reserved",
        Some("res-1"),
        None,
        None,
        "order.reserved",
        serde_json::json!({"reservation_id": "res-1"}),
    )
    .await
    .unwrap();

    assert_eq!(updated.status, "reserved");
    assert_eq!(updated.reservation_id.as_deref(), Some("res-1"));

    let count: (i64,) =
        sqlx::query_as("SELECT count(*) FROM outbox_events WHERE aggregate_id = $1::uuid")
            .bind(&row.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);

    cleanup(&state.pool).await;
}

pub async fn transition_from_wrong_state_fails(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;

    let row = sample_order("trans-key-2");
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    let err = OrderRepo::transition(
        &state.pool,
        &row.id,
        &["reserved"],
        "confirmed",
        None,
        None,
        None,
        "order.confirmed",
        serde_json::json!({}),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, OrderError::InvalidState(_)));

    cleanup(&state.pool).await;
}

pub async fn list_expired_returns_only_due_orders(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let mut expired = sample_order("exp-1");
    expired.expires_at = Utc::now() - Duration::seconds(60);
    OrderRepo::insert(&state.pool, &expired, &[]).await.unwrap();

    let mut fresh = sample_order("exp-2");
    fresh.expires_at = Utc::now() + Duration::seconds(3600);
    OrderRepo::insert(&state.pool, &fresh, &[]).await.unwrap();

    let mut done = sample_order("exp-3");
    done.status = "confirmed".into();
    done.expires_at = Utc::now() - Duration::seconds(60);
    OrderRepo::insert(&state.pool, &done, &[]).await.unwrap();

    let rows = OrderRepo::list_expired(&state.pool, 100).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, expired.id);

    cleanup(&state.pool).await;
}
