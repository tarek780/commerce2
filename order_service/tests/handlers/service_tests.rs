// grpc
use crate::common::*;

use chrono::{Duration, Utc};
use order_service::order::repo::{OrderRepo, OrderRow};
use order_service::order::service::OrderSvc;
use order_service::pb_order::order_service_server::OrderService;
use order_service::pb_order::{CancelOrderRequest, CreateOrderRequest, GetOrderRequest};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::Request;
use uuid::Uuid;

pub async fn get_order_not_found_returns_status(state: Arc<TestAppState>) {
    cleanup(&state.pool.clone()).await;

    //
    let svc = OrderSvc::new(state.appstate.clone());

    let err = svc
        .get_order(Request::new(GetOrderRequest {
            order_id: "no-such".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);

    cleanup(&state.pool).await;
}

pub async fn get_order_returns_fields(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;

    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "cust-42".into(),
        idempotency_key: "svc-get-1".into(),
        status: "pending".into(),
        total_amount: BigDecimal::from_str("12.34").unwrap(),
        currency: "USD".into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(600),
        created_at: now,
        updated_at: now,
    };
    OrderRepo::insert(&state.pool, &row, &[]).await.unwrap();

    let svc = OrderSvc::new(state.appstate.clone());
    let resp = svc
        .get_order(Request::new(GetOrderRequest {
            order_id: row.id.clone(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.id, row.id);
    assert_eq!(resp.customer_id, "cust-42");
    assert_eq!(resp.status, "pending");
    assert_eq!(resp.total_amount, "12.34");
    assert_eq!(resp.currency, "USD");

    cleanup(&state.pool).await;
}

pub async fn create_order_empty_items_invalid_argument(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;

    let svc = OrderSvc::new(state.appstate.clone());

    let err = svc
        .create_order(Request::new(CreateOrderRequest {
            customer_id: "c".into(),
            currency: "USD".into(),
            items: vec![],
            idempotency_key: "k-svc-empty".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);

    cleanup(&state.pool).await;
}

pub async fn create_order_bad_price_invalid_argument(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let svc = OrderSvc::new(state.appstate.clone());

    use order_service::pb_order::OrderItem;
    let err = svc
        .create_order(Request::new(CreateOrderRequest {
            customer_id: "c".into(),
            currency: "USD".into(),
            items: vec![OrderItem {
                variant_id: Uuid::now_v7().to_string(),
                quantity: 1,
                unit_price: "not-a-number".into(),
            }],
            idempotency_key: "k-svc-bad".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);

    cleanup(&state.pool).await;
}

pub async fn cancel_order_final_state_returns_failed_precondition(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;
    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: "svc-cancel-final".into(),
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

    let svc = OrderSvc::new(state.appstate.clone());
    let err = svc
        .cancel_order(Request::new(CancelOrderRequest {
            order_id: row.id.clone(),
            reason: "user".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);

    cleanup(&state.pool).await;
}

pub async fn cancel_order_pending_success(state: Arc<TestAppState>) {
    cleanup(&state.pool).await;

    let now = Utc::now();
    let row = OrderRow {
        id: uuid::Uuid::now_v7().to_string(),
        customer_id: "c".into(),
        idempotency_key: "svc-cancel-pending".into(),
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

    let svc = OrderSvc::new(state.appstate.clone());
    let resp = svc
        .cancel_order(Request::new(CancelOrderRequest {
            order_id: row.id.clone(),
            reason: "user".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(resp.status, "cancelled");

    cleanup(&state.pool).await;
}
