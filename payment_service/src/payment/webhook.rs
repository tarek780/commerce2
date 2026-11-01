use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

use crate::payment::repo::PaymentRepo;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct WebhookBody {
    pub event_id: String,
    pub event_type: String,
    pub payment_id: Option<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Path(payment_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let sig = headers
        .get("x-gateway-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !state.gateway.verify_webhook(&body, sig) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"bad signature"})),
        )
            .into_response();
    }

    let parsed = match serde_json::from_slice::<WebhookBody>(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    //

    let inserted: Option<String> = match sqlx::query_scalar(
        r#"INSERT INTO processed_webhooks (gateway_event_id, gateway, payment_id, event_type)
           VALUES ($1,$2,$3,$4)
           ON CONFLICT (gateway_event_id) DO NOTHING
           RETURNING gateway_event_id"#,
    )
    .bind(&parsed.event_id)
    .bind(&state.config.gateway)
    .bind(&payment_id)
    .bind(&parsed.event_type)
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!(?e, "insert processed_webhooks failed");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":"db"})),
            )
                .into_response();
        }
    };

    if inserted.is_none() {
        if let Ok(row) = PaymentRepo::get(&state.db, payment_id.clone()).await {
            return (
                StatusCode::OK,
                Json(serde_json::json!({"status": row.status})),
            )
                .into_response();
        }
        return (
            StatusCode::OK,
            Json(serde_json::json!({"status": "duplicate"})),
        )
            .into_response();
    }

    // transition
    let (from, to, evt) = match parsed.event_type.as_str() {
        "payment.succeeded" => (
            vec!["pending", "processing"],
            "succeeded",
            "payment.succeeded",
        ),
        "payment.failed" => (vec!["pending", "processing"], "failed", "payment.failed"),
        "payment.refunded" => (vec!["succeeded"], "refunded", "payment.refunded"),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"unknown event"})),
            )
                .into_response()
        }
    };

    match PaymentRepo::transition(
        &state.db,
        payment_id.clone(),
        &from,
        to,
        None,
        None,
        None,
        evt,
        serde_json::json!({"payment_id": payment_id, "event": parsed.event_type}),
    )
    .await
    {
        Ok(row) => {
            state
                .try_publish(
                    evt,
                    row.id,
                    Some(row.order_id),
                    &row.status,
                    row.failure_reason.as_deref(),
                    serde_json::json!({"gateway_event_id": parsed.event_id}),
                )
                .await;

            (
                StatusCode::OK,
                Json(serde_json::json!({"status": row.status})),
            )
                .into_response()
        }

        Err(e) => {
            warn!(?e, "transition failed");
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ignored", "reason": format!("{e:?}")})),
            )
                .into_response()
        }
    }
}
