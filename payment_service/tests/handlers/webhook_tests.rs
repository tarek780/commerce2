use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use payment_service::payment::repo::{PaymentRepo, PaymentRow};
use payment_service::payment::webhook;
use sha2::Sha256;
use sqlx::types::BigDecimal;
use std::str::FromStr;
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

pub async fn webhook_rejects_bad_signature(
    state: &std::sync::Arc<payment_service::state::AppState>,
) {
    let id = seed_payment(&state, "wh-bad").await;

    let body = Bytes::from_static(br#"{"event_id":"e1","event_type":"payment.succeeded"}"#);

    let resp = webhook::handle(
        State(state.clone()),
        Path(id),
        headers_with_sig("deadbeef"),
        body,
    )
    .await
    .into_response();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

pub async fn webhook_succeeded_transitions_payment(
    state: &std::sync::Arc<payment_service::state::AppState>,
) {
    let id = seed_payment(&state, "wh-ok").await;

    let raw = br#"{"event_id":"evt-1","event_type":"payment.succeeded"}"#;
    let sig = sign(&state.config.webhook_secret, raw);

    let resp = webhook::handle(
        State(state.clone()),
        Path(id.clone()),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), StatusCode::OK);

    let row = PaymentRepo::get(&state.db, id).await.unwrap();
    assert_eq!(row.status, "succeeded");
}

pub async fn webhook_is_idempotent_on_duplicate_event_id(
    state: &std::sync::Arc<payment_service::state::AppState>,
) {
    let id = seed_payment(&state, "wh-dup").await;

    let raw = br#"{"event_id":"evt-dup","event_type":"payment.succeeded"}"#;
    let sig = sign(&state.config.webhook_secret, raw);

    let r1 = webhook::handle(
        State(state.clone()),
        Path(id.clone()),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();
    assert_eq!(r1.status(), StatusCode::OK);

    let r2 = webhook::handle(
        State(state.clone()),
        Path(id.clone()),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();
    assert_eq!(r2.status(), StatusCode::OK);

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM outbox_events WHERE aggregate_id = $1")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

pub async fn webhook_unknown_event_is_rejected(
    state: &std::sync::Arc<payment_service::state::AppState>,
) {
    let id = seed_payment(&state, "wh-unknown").await;

    let raw = br#"{"event_id":"evt-u","event_type":"payment.wat"}"#;
    let sig = sign(&state.config.webhook_secret, raw);

    let resp = webhook::handle(
        State(state.clone()),
        Path(id),
        headers_with_sig(&sig),
        Bytes::from_static(raw),
    )
    .await
    .into_response();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

pub async fn webhook_master(state: std::sync::Arc<payment_service::state::AppState>) {
    webhook_is_idempotent_on_duplicate_event_id(&state).await;
    webhook_rejects_bad_signature(&state).await;
    webhook_succeeded_transitions_payment(&state).await;
    webhook_unknown_event_is_rejected(&state).await;
}
