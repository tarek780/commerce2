
mod common;

use chrono::{Duration, Utc};
use payment_service::payment::repo::{PaymentRepo, PaymentRow};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use uuid::Uuid;

fn sample_row(order_id: &str) -> PaymentRow {
    let now = Utc::now();
    PaymentRow {
        id: Uuid::now_v7().to_string(),
        order_id: order_id.into(),
        idempotency_key: format!("idem-{order_id}"),
        amount: BigDecimal::from_str("100.00").unwrap(),
        currency: "USD".into(),
        status: "pending".into(),
        gateway: "mock".into(),
        gateway_reference: None,
        payment_url: None,
        failure_reason: None,
        customer_id: None,
        metadata: serde_json::json!({}),
        reservation_expires_at: now + Duration::seconds(900),
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
#[ignore = "requires DB"]
async fn insert_and_get_roundtrip() {
    let st = common::test_state().await;
    let row = sample_row("order-rt");

    PaymentRepo::insert(&st.db, &row).await.unwrap();
    let got = PaymentRepo::get(&st.db, row.id.clone()).await.unwrap();

    assert_eq!(got.id, row.id);
    assert_eq!(got.order_id, "order-rt");
    assert_eq!(got.status, "pending");
}

#[tokio::test]
#[ignore = "requires DB"]
async fn insert_duplicate_idempotency_returns_duplicate() {
    let st = common::test_state().await;
    let a = sample_row("order-dup");
    let b = sample_row("order-dup"); // same idempotency_key

    PaymentRepo::insert(&st.db, &a).await.unwrap();
    let err = PaymentRepo::insert(&st.db, &b).await.unwrap_err();

    assert!(matches!(err, payment_service::error::PaymentError::Duplicate));
}

#[tokio::test]
#[ignore = "requires DB"]
async fn transition_pending_to_processing_creates_outbox() {
    let st = common::test_state().await;
    let row = sample_row("order-tr");
    PaymentRepo::insert(&st.db, &row).await.unwrap();

    let updated = PaymentRepo::transition(
        &st.db,
        row.id.clone(),
        &["pending"],
        "processing",
        Some("ref-1"),
        Some("http://pay/ref-1"),
        None,
        "payment.created",
        serde_json::json!({"payment_id": row.id}),
    )
    .await
    .unwrap();

    assert_eq!(updated.status, "processing");
    assert_eq!(updated.gateway_reference.as_deref(), Some("ref-1"));

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox_events WHERE aggregate_id = $1")
            .bind(&row.id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[ignore = "requires DB"]
async fn transition_from_wrong_state_fails() {
    let st = common::test_state().await;
    let row = sample_row("order-bad");
    PaymentRepo::insert(&st.db, &row).await.unwrap();


    let err = PaymentRepo::transition(
        &st.db,
        row.id.clone(),
        &["processing"],
        "succeeded",
        None,
        None,
        None,
        "payment.succeeded",
        serde_json::json!({}),
    )
    .await
    .unwrap_err();

    assert!(matches!(
        err,
        payment_service::error::PaymentError::InvalidState(_)
    ));
}

#[tokio::test]
#[ignore = "requires DB"]
async fn find_expired_returns_only_old_pending() {
    let st = common::test_state().await;


    let mut expired = sample_row("order-exp");
    expired.reservation_expires_at = Utc::now() - Duration::seconds(60);
    PaymentRepo::insert(&st.db, &expired).await.unwrap();


    let fresh = sample_row("order-fresh");
    PaymentRepo::insert(&st.db, &fresh).await.unwrap();

    let rows = PaymentRepo::find_expired(&st.db, 100).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, expired.id);
}