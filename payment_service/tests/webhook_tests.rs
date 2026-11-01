
mod common;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use payment_service::payment::repo::{PaymentRepo, PaymentRow};
use payment_service::payment::webhook;
use sha2::Sha256;
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

async fn seed_payment(state: &Arc<payment_service::state::AppState>, order: &str) -> String {
    let now = Utc::now();
    let row = PaymentRow {
        id: Uuid::now_v7().to_string(),
        order_id: order.into(),
        idempotency_key: format!("idem-{order}"),
        amount: BigDecimal::from_str("10.00").unwrap(),
        currency: "USD".into(),
        status: "processing".into(),
        gateway: "mock".into(),
        gateway_reference: Some("ref-x".into()),
        payment_url: Some("http://x".into()),
        failure_reason: None,
        customer_id: None,
        metadata: serde_json::json!({}),
        reservation_expires_at: now + Duration::seconds(900),
        created_at: now,
        updated_at: now,
    };
    PaymentRepo::insert(&state.db, &row).await.unwrap();
    row.id
}

fn headers_with_sig(sig: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert("x-gateway-signature", HeaderValue::from_str(sig).unwrap());
    h
}

#[tokio::test]
#[ignore = "requires DB"]
async fn webhook_rejects_bad_signature() {
    let st = common::test_state().await;
    let id = seed_payment(&st, "wh-bad").await;

    let body = Bytes::from_static(
        br#"{"event_id":"e1","event_type":"payment.succeeded"}"#,
    );

    let resp = webhook::handle(
        State(st.clone()),
        Path(id),
        headers_with_sig("deadbeef"),
        body,
    )
    .await
    .into_response();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[ignore = "requires DB"]
async fn webhook_succeeded_transitions_payment() {
    let st = common::test_state().await;
    let id = seed_payment(&st, "wh-ok").await;

    let raw = br#"{"event_id":"evt-1","event_type":"payment.succeeded"}"#;
    let sig = sign(&st.config.webhook_secret, raw);

    let resp = webhook::handle(
        State(st.clone()),
        Path(id.clone()),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), StatusCode::OK);

    let row = PaymentRepo::get(&st.db, id).await.unwrap();
    assert_eq!(row.status, "succeeded");
}

#[tokio::test]
#[ignore = "requires DB"]
async fn webhook_is_idempotent_on_duplicate_event_id() {
    let st = common::test_state().await;
    let id = seed_payment(&st, "wh-dup").await;

    let raw = br#"{"event_id":"evt-dup","event_type":"payment.succeeded"}"#;
    let sig = sign(&st.config.webhook_secret, raw);


    let r1 = webhook::handle(
        State(st.clone()),
        Path(id.clone()),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();
    assert_eq!(r1.status(), StatusCode::OK);


    let r2 = webhook::handle(
        State(st.clone()),
        Path(id.clone()),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();
    assert_eq!(r2.status(), StatusCode::OK);

    // outbox يجب أن يحتوي سجل واحد فقط
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox_events WHERE aggregate_id = $1")
            .bind(&id)
            .fetch_one(&st.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[ignore = "requires DB"]
async fn webhook_unknown_event_is_rejected() {
    let st = common::test_state().await;
    let id = seed_payment(&st, "wh-unknown").await;

    let raw = br#"{"event_id":"evt-u","event_type":"payment.wat"}"#;
    let sig = sign(&st.config.webhook_secret, raw);

    let resp = webhook::handle(
        State(st.clone()),
        Path(id),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}