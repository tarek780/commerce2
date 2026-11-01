use axum::{
    extract::{State , Extension},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use bigdecimal::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::dto::checkout::CheckoutResponse;
use crate::error::{AppError, AppResult};
use crate::helpers::idempotency::IdempotencyStore;
use crate::middleware::auth::AccessToken;
use crate::state::AppState;

/// POST /api/v1/checkout
pub async fn checkout(
    State(state): State<Arc<AppState>>,
   Extension(auth): Extension<AccessToken>,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
//
let user_id = uuid::Uuid::parse_str(&auth.user_id)
        .map_err(|_| AppError::Unauthorized("invalid user_id in token".into()))?;
//
    let idem_header = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok());

    let (idem_key, from_header) =
        IdempotencyStore::key_from_header_or_new(idem_header);

    if state.cfg.idempotency_required && !from_header {
        return Err(AppError::Validation(
            "Idempotency-Key header is required".into(),
        ));
    }

    let store = IdempotencyStore::new(state.cfg.idempotency_ttl_secs);
    let mut redis = state.redis.clone();

    // 1) idempotency
    if let Some(cached) = store.try_reserve(&mut redis, &idem_key).await? {
        tracing::info!(user_id = %user_id, "checkout idempotency hit");
        let parsed: CheckoutResponse = serde_json::from_str(&cached)
            .map_err(|e| AppError::Internal(format!("idem decode: {e}")))?;
        return Ok((StatusCode::CREATED, Json(parsed)));
    }

    // 2) cart
    let cart = state.cart.get_cart(user_id).await?;
    if cart.items.is_empty() {
        return Err(AppError::Cart("cart is empty".into()));
    }

    // 3) resolve prices من Category (client)
    let mut order_items: Vec<(i64, i32, String)> = Vec::with_capacity(cart.items.len());
    for item in &cart.items {
        if item.quantity <= 0 {
            return Err(AppError::Validation(format!(
                "invalid quantity for variant {}",
                item.variant_id
            )));
        }
        let price: BigDecimal = state.category.get_variant_price(item.variant_id).await?;
        order_items.push((item.variant_id, item.quantity, price.to_string()));
    }

    // 4) create order (gRPC)
    let order = state
        .order
        .create_order(
            &user_id.to_string(),
            &state.cfg.currency,
            order_items,
            &format!("checkout:{idem_key}"),
        )
        .await?;

    // 5) clear cart (best-effort)
    let mut warnings = Vec::new();
    if let Err(e) = state.cart.clear_cart(user_id).await {
        tracing::warn!(error = %e, "cart clear failed after checkout");
        warnings.push("cart_not_cleared".to_string());
    }

    let resp = CheckoutResponse {
        order_id: order.id.clone(),
        status: order.status.clone(),
        total_amount: order.total_amount.clone(),
        currency: order.currency.clone(),
        payment_id: order.payment_id.clone(),
        payment_url: extract_payment_url(&order.payment_id, &state).await,
        warnings,
    };

    // 6) store idempotent result
    let body = serde_json::to_string(&resp)
        .map_err(|e| AppError::Internal(format!("idem encode: {e}")))?;
    let _ = store.store_result(&mut redis, &idem_key, &body).await;

    Ok((StatusCode::CREATED, Json(resp)))
}



async fn extract_payment_url(payment_id: &str, _state: &AppState) -> String {
    if payment_id.is_empty() {
        String::new()
    } else {


        format!("/api/v1/orders/{payment_id}/pay")
    }
}

// أضف في بداية الملف
use crate::helpers::idempotency::{key_from_header_or_new};

// غيّر جسم checkout:
pub async fn checkoutt(
    State(state): State<Arc<AppState>>,
    CurrentUser(user_id): CurrentUser,
    headers: HeaderMap,
) -> AppResult<impl IntoResponse> {
    let idem_header = headers.get("idempotency-key").and_then(|v| v.to_str().ok());
    let (idem_key, from_header) = key_from_header_or_new(idem_header);

    if state.cfg.idempotency_required && !from_header {
        return Err(AppError::Validation("Idempotency-Key header is required".into()));
    }

    // idempotency через trait
    if let Some(cached) = state.idempotency.try_reserve(&idem_key).await? {
        let parsed: CheckoutResponse = serde_json::from_str(&cached)
            .map_err(|e| AppError::Internal(format!("idem decode: {e}")))?;
        return Ok((StatusCode::CREATED, Json(parsed)));
    }

    // ... باقي المنطق كما هو ...
    // في النهاية:
    let body = serde_json::to_string(&resp)
        .map_err(|e| AppError::Internal(format!("idem encode: {e}")))?;
    let _ = state.idempotency.store_result(&idem_key, &body).await;
    Ok((StatusCode::CREATED, Json(resp)))
}