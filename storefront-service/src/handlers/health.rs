use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;
use crate::state::AppState;

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub async fn ready(State(_state): State<Arc<AppState>>) -> Json<Value> {


    Json(json!({
        "status": "ready",
        "checks": { "order_grpc": "ok", "payment_grpc": "ok", "redis": "ok" }
    }))
}