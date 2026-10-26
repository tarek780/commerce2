use axum::{
    Extension, Json,
    extract::{Path, State},
};
use std::sync::Arc;

use crate::auth::layer::AccessToken;
use crate::cart::model::*;
use crate::error::CartError;
use crate::state::AppState;

// ---- Guest ----
#[tracing::instrument(skip(state))]
pub async fn create_guest_cart(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GuestCartCreated>, CartError> {
    let guest_cart_id = state.cart_service.create_guest_cart().await?;
    tracing::info!(guest_cart_id, "guest cart created");
    Ok(Json(GuestCartCreated { guest_cart_id }))
}

#[tracing::instrument(skip(state))]
pub async fn get_guest_cart(
    State(state): State<Arc<AppState>>,
    Path(guest_id): Path<String>,
) -> Result<Json<CartResponse>, CartError> {
    let c = state.cart_service.get_guest_cart(&guest_id).await?;
    Ok(Json(CartResponse {
        items: c.items,
        updated_at: c.updated_at,
    }))
}

#[tracing::instrument(skip(state), fields(guest_id = %guest_id))]
pub async fn add_guest_item(
    State(state): State<Arc<AppState>>,
    Path(guest_id): Path<String>,
    Json(req): Json<AddItemRequest>,
) -> Result<Json<CartResponse>, CartError> {
    state.cart_service.add_guest_item(&guest_id, req).await?;
    let c = state.cart_service.get_guest_cart(&guest_id).await?;
    Ok(Json(CartResponse {
        items: c.items,
        updated_at: c.updated_at,
    }))
}

// ---- User ----
#[tracing::instrument(skip(state), fields(user_id = %auth.user_id))]
pub async fn get_user_cart(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AccessToken>,
) -> Result<Json<CartResponse>, CartError> {
    let c = state.cart_service.get_user_cart(&auth.user_id).await?;
    Ok(Json(CartResponse {
        items: c.items,
        updated_at: c.updated_at,
    }))
}

#[tracing::instrument(skip(state), fields(user_id = %auth.user_id))]
pub async fn add_user_item(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AccessToken>,
    Json(req): Json<AddItemRequest>,
) -> Result<Json<CartResponse>, CartError> {
    state.cart_service.add_user_item(&auth.user_id, req).await?;
    let c = state.cart_service.get_user_cart(&auth.user_id).await?;
    Ok(Json(CartResponse {
        items: c.items,
        updated_at: c.updated_at,
    }))
}

/// POST /api/cart/items/bulk — إضافة مجموعة منتجات دفعة واحدة
#[tracing::instrument(skip(state, items), fields(user_id = %auth.user_id, count = items.len()))]
pub async fn add_user_items_bulk(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AccessToken>,
    Json(items): Json<Vec<AddItemRequest>>,
) -> Result<Json<CartResponse>, CartError> {
    state
        .cart_service
        .add_user_items(&auth.user_id, items)
        .await?;
    let c = state.cart_service.get_user_cart(&auth.user_id).await?;
    Ok(Json(CartResponse {
        items: c.items,
        updated_at: c.updated_at,
    }))
}

// ---- Merge ----
#[tracing::instrument(
    skip(state, req),
    fields(user_id = %auth.user_id, guest_id = %req.guest_cart_id)
)]
pub async fn merge_cart(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AccessToken>,
    Json(req): Json<MergeRequest>,
) -> Result<Json<CartResponse>, CartError> {
    state
        .cart_service
        .merge_guest_into_user(&auth.user_id, &req.guest_cart_id)
        .await?;
    let c = state.cart_service.get_user_cart(&auth.user_id).await?;
    Ok(Json(CartResponse {
        items: c.items,
        updated_at: c.updated_at,
    }))
}
