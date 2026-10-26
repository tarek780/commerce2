use axum::{Json, extract::State};
use std::sync::Arc;

use crate::{
    db::user_repo,
    dto::auth::{MeResponse, UpdateMeRequest},
    error::AppError,
    helpers::validation,
    middleware::auth_layer::AuthUser,
    state::AppState,
};

pub async fn get_me(
    State(state): State<Arc<AppState>>,
    AuthUser { user_id }: AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let user = user_repo::find_by_id(&state.pool, &user_id.as_str())
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        email_verified: user.email_verified_at.is_some(),
        created_at: user.created_at,
    }))
}

pub async fn update_me(
    State(state): State<Arc<AppState>>,
    AuthUser { user_id }: AuthUser,
    Json(payload): Json<UpdateMeRequest>,
) -> Result<Json<MeResponse>, AppError> {
    validation::validate(&payload)?;

    let user = user_repo::update_display_name(
        &state.pool,
        &user_id.as_str(),
        payload.display_name.as_deref(),
    )
    .await?;

    Ok(Json(MeResponse {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        email_verified: user.email_verified_at.is_some(),
        created_at: user.created_at,
    }))
}
