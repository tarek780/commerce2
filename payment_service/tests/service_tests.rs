// grpc

mod common;

use payment_service::pb::payment_service_server::PaymentService;
use payment_service::pb::{CreatePaymentRequest, GetPaymentRequest, CancelPaymentRequest};
use payment_service::payment::service::PaymentSvc;
use tonic::Request;

fn create_req(order: &str, idem: &str) -> CreatePaymentRequest {
    CreatePaymentRequest {
        order_id: order.into(),
        amount: "100.00".into(),
        currency: "USD".into(),
        idempotency_key: idem.into(),
        customer_id: String::new(),
        metadata: Default::default(),
    }
}

#[tokio::test]
#[ignore = "requires DB"]
async fn create_payment_persists_and_returns_url() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());

    let resp = svc
        .create_payment(Request::new(create_req("order-1", "idem-1")))
        .await
        .unwrap()
        .into_inner();

    assert!(!resp.id.is_empty());
    assert_eq!(resp.status, "processing");
    assert!(!resp.payment_url.is_empty());
    assert!(resp.gateway_reference.starts_with("mock_"));
}

#[tokio::test]
#[ignore = "requires DB"]
async fn create_payment_is_idempotent_on_same_key() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());

    let a = svc
        .create_payment(Request::new(create_req("order-i", "idem-same")))
        .await
        .unwrap()
        .into_inner();

    let b = svc
        .create_payment(Request::new(create_req("order-i", "idem-same")))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(a.id, b.id, "idempotent calls should return same payment");
}

#[tokio::test]
#[ignore = "requires DB"]
async fn create_payment_idempotency_mismatch_fails() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());

    svc.create_payment(Request::new(create_req("order-m", "idem-m")))
        .await
        .unwrap();


    let err = svc
        .create_payment(Request::new(create_req("order-OTHER", "idem-m")))
        .await
        .unwrap_err();

    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
#[ignore = "requires DB"]
async fn get_payment_not_found_returns_not_found() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());

    let err = svc
        .get_payment(Request::new(GetPaymentRequest {
            payment_id: "00000000-0000-0000-0000-000000000000".into(),
        }))
        .await
        .unwrap_err();

    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
#[ignore = "requires DB"]
async fn cancel_payment_from_processing_succeeds() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());

    let created = svc
        .create_payment(Request::new(create_req("order-c", "idem-c")))
        .await
        .unwrap()
        .into_inner();

    let cancelled = svc
        .cancel_payment(Request::new(CancelPaymentRequest {
            payment_id: created.id.clone(),
            reason: "user_request".into(),
        }))
        .await
        .unwrap()
        .into_inner();

    assert_eq!(cancelled.status, "cancelled");
}

#[tokio::test]
#[ignore = "requires DB"]
async fn refund_requires_succeeded_state() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());


    let created = svc
        .create_payment(Request::new(create_req("order-rf1", "idem-rf1")))
        .await.unwrap().into_inner();

    let err = svc
        .refund_payment(Request::new(RefundPaymentRequest {
            payment_id: created.id.clone(),
            amount: "10.00".into(),
            reason: "test".into(),
        }))
        .await.unwrap_err();

    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
}

#[tokio::test]
#[ignore = "requires DB"]
async fn refund_succeeded_payment_marks_refunded() {
    let st = common::test_state().await;
    let svc = PaymentSvc::new(st.clone());

    let created = svc
        .create_payment(Request::new(create_req("order-rf2", "idem-rf2")))
        .await.unwrap().into_inner();


    sqlx::query("UPDATE payments SET status='succeeded' WHERE id=$1")
        .bind(&created.id).execute(&st.db).await.unwrap();

    let refunded = svc
        .refund_payment(Request::new(RefundPaymentRequest {
            payment_id: created.id.clone(),
            amount: "10.00".into(),
            reason: "customer".into(),
        }))
        .await.unwrap().into_inner();

    assert_eq!(refunded.status, "refunded");
}