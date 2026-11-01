mod common;

use chrono::{Duration, Utc};
use order_service::error::OrderError;
use order_service::order::repo::{OrderRepo, OrderRow};
use sqlx::types::BigDecimal;
use std::str::FromStr;
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

#[tokio::test]
async fn insert_and_get_order() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;

    let row = sample_order("key-1");
    let items = vec![(101, 2, BigDecimal::from_str("5.25").unwrap())];

    OrderRepo::insert(&pool, &row, &items).await.unwrap();
    let got = OrderRepo::get(&pool, &row.id).await.unwrap();
    assert_eq!(got.id, row.id);
    assert_eq!(got.status, "pending");
    assert_eq!(got.total_amount, row.total_amount);

    let got_items = OrderRepo::items(&pool, &row.id).await.unwrap();
    assert_eq!(got_items.len(), 1);
    assert_eq!(got_items[0].variant_id, 101);
    assert_eq!(got_items[0].quantity, 2);

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn duplicate_idempotency_key_returns_duplicate() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;

    let r1 = sample_order("dup-key");
    OrderRepo::insert(&pool, &r1, &[]).await.unwrap();

    let r2 = sample_order("dup-key");
    let err = OrderRepo::insert(&pool, &r2, &[]).await.unwrap_err();
    assert!(matches!(err, OrderError::Duplicate));

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn get_missing_returns_not_found() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;

    let err = OrderRepo::get(&pool, "missing-id").await.unwrap_err();
    assert!(matches!(err, OrderError::NotFound));

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn transition_pending_to_reserved() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;

    let row = sample_order("trans-key-1");
    OrderRepo::insert(&pool, &row, &[]).await.unwrap();

    let updated = OrderRepo::transition(
        &pool,
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
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn transition_from_wrong_state_fails() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;

    let row = sample_order("trans-key-2");
    OrderRepo::insert(&pool, &row, &[]).await.unwrap();


    let err = OrderRepo::transition(
        &pool,
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

    common::cleanup(&pool).await;
}

#[tokio::test]
async fn list_expired_returns_only_due_orders() {
    let pool = common::setup_pool().await;
    common::cleanup(&pool).await;


    let mut expired = sample_order("exp-1");
    expired.expires_at = Utc::now() - Duration::seconds(60);
    OrderRepo::insert(&pool, &expired, &[]).await.unwrap();


    let mut fresh = sample_order("exp-2");
    fresh.expires_at = Utc::now() + Duration::seconds(3600);
    OrderRepo::insert(&pool, &fresh, &[]).await.unwrap();


    let mut done = sample_order("exp-3");
    done.status = "confirmed".into();
    done.expires_at = Utc::now() - Duration::seconds(60);
    OrderRepo::insert(&pool, &done, &[]).await.unwrap();

    let rows = OrderRepo::list_expired(&pool, 100).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, expired.id);

    common::cleanup(&pool).await;
}