use crate::common::*;

use order_service::order::orchestrator::{self, NewItem};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

fn new_item(v: Uuid, q: i32, price: &str) -> NewItem {
    NewItem {
        variant_id: v,
        quantity: q,
        unit_price: BigDecimal::from_str(price).unwrap(),
    }
}

pub async fn empty_items_rejected(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;
    //

    let err =
        orchestrator::create_and_orchestrate(&state.appstate, "cust", "USD", vec![], "idem-1")
            .await
            .unwrap_err();

    assert!(matches!(
        err,
        order_service::error::OrderError::InvalidState(_)
    ));
    cleanup(&state.pool.clone()).await;
}

pub async fn cancel_final_state_rejected(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;
    //

    use chrono::{Duration, Utc};
    use order_service::order::repo::{OrderRepo, OrderRow};
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
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    let err = orchestrator::cancel(&state.appstate, &row.id, "test")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        order_service::error::OrderError::InvalidState(_)
    ));

    cleanup(&state.pool.clone()).await;
}

pub async fn cancel_pending_succeeds_without_clients(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

    use chrono::{Duration, Utc};
    use order_service::order::repo::{OrderRepo, OrderRow};
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
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    let out = orchestrator::cancel(&state.appstate, &row.id, "user_cancel")
        .await
        .unwrap();
    assert_eq!(out.status, "cancelled");
    assert_eq!(out.failure_reason.as_deref(), Some("user_cancel"));

    cleanup(&state.pool.clone()).await;
}

pub async fn happy_path_full_orchestration(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

    let items = vec![new_item(uuid::Uuid::now_v7(), 1, "9.99")];
    let row =
        orchestrator::create_and_orchestrate(&state.appstate, "c1", "USD", items, "idem-happy")
            .await
            .unwrap();

    assert_eq!(row.status, "awaiting_payment");
    assert!(row.reservation_id.is_some());
    assert!(row.payment_id.is_some());

    cleanup(&state.pool.clone()).await;
}
