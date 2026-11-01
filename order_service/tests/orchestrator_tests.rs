mod common;

use order_service::clients::inventory::InventoryClient;
use order_service::clients::payment::PaymentClient;
use order_service::order::orchestrator::{self, NewItem};
use order_service::state::AppState;
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::transport::Endpoint;
use uuid::Uuid;

fn new_item(v: Uuid, q: i32, price: &str) -> NewItem {
    NewItem {
        variant_id: v,
        quantity: q,
        unit_price: BigDecimal::from_str(price).unwrap(),
    }
}

#[tokio::test]
async fn empty_items_rejected() {
    let state = common::setup_pool().await;
    common::cleanup(&state.0.clone()).await;
    let state = common::setup_state(state.0, state.1, state.2).await;
    //

    let err = orchestrator::create_and_orchestrate(&state, "cust", "USD", vec![], "idem-1")
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        order_service::error::OrderError::InvalidState(_)
    ));
    common::cleanup(&state.db.clone()).await;
}

#[tokio::test]
async fn cancel_final_state_rejected() {
    let state = common::setup_pool().await;
    common::cleanup(&state.0.clone()).await;
    let state = common::setup_state(state.0, state.1, state.2).await;
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
    OrderRepo::insert(&state.db, &row, &[]).await.unwrap();

    let err = orchestrator::cancel(&state, &row.id, "test")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        order_service::error::OrderError::InvalidState(_)
    ));

    common::cleanup(&state.db.clone()).await;
}

#[tokio::test]
async fn cancel_pending_succeeds_without_clients() {
    let state = common::setup_pool().await;
    common::cleanup(&state.0.clone()).await;
    let state = common::setup_state(state.0, state.1, state.2).await;

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
    OrderRepo::insert(&state.db, &row, &[]).await.unwrap();

    let out = orchestrator::cancel(&state, &row.id, "user_cancel")
        .await
        .unwrap();
    assert_eq!(out.status, "cancelled");
    assert_eq!(out.failure_reason.as_deref(), Some("user_cancel"));

    common::cleanup(&state.db.clone()).await;
}

#[tokio::test]
#[ignore = "requires running inventory & payment services"]
async fn happy_path_full_orchestration() {
    let state = common::setup_pool().await;
    common::cleanup(&state.0.clone()).await;
    let state = common::setup_state(state.0, state.1, state.2).await;

    let items = vec![new_item(uuid::Uuid::now_v7(), 1, "9.99")];
    let row = orchestrator::create_and_orchestrate(&state, "c1", "USD", items, "idem-happy")
        .await
        .unwrap();

    assert_eq!(row.status, "awaiting_payment");
    assert!(row.reservation_id.is_some());
    assert!(row.payment_id.is_some());

    common::cleanup(&state.db.clone()).await;
}
