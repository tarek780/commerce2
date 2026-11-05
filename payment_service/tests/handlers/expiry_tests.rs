// tests/expiry_tests.rs
use std::sync::Arc;

use chrono::{Duration, Utc};
use payment_service::payment::expiry::expire_batch;
use payment_service::payment::repo::{PaymentRepo, PaymentRow};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use uuid::Uuid;

async fn seed_expired(state: &std::sync::Arc<payment_service::state::AppState>) -> String {
    let now = Utc::now();
    let row = PaymentRow {
        id: Uuid::now_v7().to_string(),
        order_id: format!("exp-{}", Uuid::now_v7()),
        idempotency_key: format!("idem-{}", Uuid::now_v7()),
        amount: BigDecimal::from_str("5.00").unwrap(),
        currency: "USD".into(),
        status: "pending".into(),
        gateway: "mock".into(),
        gateway_reference: Some("ref-exp".into()),
        payment_url: None,
        failure_reason: None,
        customer_id: None,
        metadata: serde_json::json!({}),
        reservation_expires_at: now - Duration::seconds(60),
        created_at: now - Duration::seconds(120),
        updated_at: now - Duration::seconds(120),
    };
    PaymentRepo::insert(&state.db, &row).await.unwrap();
    row.id
}

pub async fn expire_batch_marks_pending_as_expired(state: Arc<payment_service::state::AppState>) {
    let id = seed_expired(&state).await;

    let n = expire_batch(&state).await.unwrap();
    assert_eq!(n, 1);

    let row = PaymentRepo::get(&state.db, id).await.unwrap();
    assert_eq!(row.status, "expired");
    assert_eq!(
        row.failure_reason.as_deref(),
        Some("reservation_ttl_exceeded")
    );
}

pub async fn expire_batch_returns_zero_when_nothing_expired(
    state: Arc<payment_service::state::AppState>,
) {
    let n = expire_batch(&state).await.unwrap();
    assert_eq!(n, 0);
}

pub async fn expiry_master(state: std::sync::Arc<payment_service::state::AppState>) {
    seed_expired(&state).await;
    expire_batch_marks_pending_as_expired(state.clone()).await;
    expire_batch_returns_zero_when_nothing_expired(state.clone()).await;
}
