use axum::{
    extract::{Path, Query, State ,Extension},
    Json,
};
// Extension(auth): Extension<AccessToken>,
use std::sync::Arc;
use uuid::Uuid;

use crate::clients::order::pb_order::OrderResponse;
use crate::dto::order::{
    CancelOrderBody, OrderConnection, OrderDto, OrderEdge, PageInfoDto, PayOrderResponse,
};
use crate::dto::pagination::PageParams;
use crate::error::AppError;
use crate::middleware::user::CurrentUser;
use crate::state::AppState;

//
pub async fn get_order(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AccessToken>,
    Path(order_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let order = state.order.get_order(&order_id).await?;
    if order.customer_id != auth.user_id {
        return Err(AppError::NotFound("order not found".into()));
    }
    Ok(Json(to_order_dto(order)))
}
//
/// GET /api/v1/orders
pub async fn list(
    State(state): State<Arc<AppState>>,
    CurrentUser(user_id): CurrentUser,
    Query(page): Query<PageParams>,
) -> crate::error::AppResult<Json<OrderConnection>> {
    let res = state
        .order
        .list_orders(&user_id.to_string(), page.limit(), &page.after_str(), "")
        .await?;

    let edges: Vec<OrderEdge> = res
        .edges
        .into_iter()
        .map(|o| OrderEdge {
            cursor: encode_cursor(&o.id),
            node: to_dto(o),
        })
        .collect();

    Ok(Json(OrderConnection {
        edges,
        page_info: PageInfoDto {
            has_next_page: res.has_next_page,
            end_cursor: if res.end_cursor.is_empty() { None } else { Some(res.end_cursor) },
        },
    }))
}

/// GET /api/v1/orders/:id
pub async fn get(
    State(state): State<Arc<AppState>>,
    CurrentUser(user_id): CurrentUser,
    Path(id): Path<String>,
) -> crate::error::AppResult<Json<OrderDto>> {
    Uuid::parse_str(&id).map_err(|_| AppError::Validation("invalid order id".into()))?;
    let o = state.order.get_order(&id).await?;
    if o.customer_id != user_id.to_string() {
        return Err(AppError::NotFound("order not found".into()));
    }
    Ok(Json(to_dto(o)))
}

/// POST /api/v1/orders/:id/cancel
pub async fn cancel(
    State(state): State<Arc<AppState>>,
    CurrentUser(user_id): CurrentUser,
    Path(id): Path<String>,
    Json(body): Json<CancelOrderBody>,
) -> crate::error::AppResult<Json<OrderDto>> {
    // تحقق من الملكية أولاً
    let existing = state.order.get_order(&id).await?;
    if existing.customer_id != user_id.to_string() {
        return Err(AppError::NotFound("order not found".into()));
    }
    let o = state.order.cancel_order(&id, &body.reason).await?;
    Ok(Json(to_dto(o)))
}

/// POST /api/v1/orders/:id/pay
pub async fn pay(
    State(state): State<Arc<AppState>>,
    CurrentUser(user_id): CurrentUser,
    Path(id): Path<String>,
) -> crate::error::AppResult<Json<PayOrderResponse>> {
    let existing = state.order.get_order(&id).await?;
    if existing.customer_id != user_id.to_string() {
        return Err(AppError::NotFound("order not found".into()));
    }
    let r = state.order.pay_order(&id).await?;
    Ok(Json(PayOrderResponse {
        order_id: r.order_id,
        payment_id: r.payment_id,
        payment_url: r.payment_url,
        status: r.status,
    }))
}

fn to_dto(o: OrderResponse) -> OrderDto {
    OrderDto {
        id: o.id,
        customer_id: o.customer_id,
        status: o.status,
        total_amount: o.total_amount,
        currency: o.currency,
        reservation_id: opt(o.reservation_id),
        payment_id: opt(o.payment_id),
        created_at: o.created_at,
        updated_at: o.updated_at,
        failure_reason: o.failure_reason,
    }
}

fn opt(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

fn encode_cursor(s: &str) -> String {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes())
}