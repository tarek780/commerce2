use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use crate::dto::payment::PaymentDto;
use crate::error::{AppError, AppResult};
use crate::middleware::user::CurrentUser;
use crate::state::AppState;

/// GET /api/v1/payments/:id
#[utoipa::path(
    get,
    path = "/api/v1/payments/{:id}",
    
    responses(
        (status = 200, description = "Success"
        )
    )
)]
pub async fn get(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user_id): CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<PaymentDto>> {
    if id.is_empty() {
        return Err(AppError::Validation("payment id required".into()));
    }
    let p = state.payment.get_payment(&id).await?;
    Ok(Json(PaymentDto {
        id: p.id,
        order_id: p.order_id,
        status: p.status,
        amount: p.amount,
        currency: p.currency,
        gateway_reference: p.gateway_reference,
        payment_url: p.payment_url,
        created_at: p.created_at,
        updated_at: p.updated_at,
        failure_reason: p.failure_reason,
    }))
}