use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    db::{token_repo, user_repo},
    dto::auth::*,
    error::AppError,

    helpers::{email, email_store, jwt, password, token, validation},
    state::AppState,
};

#[tracing::instrument(skip(state, payload), fields(email = %payload.email))]
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    validation::validate(&payload)?;

    let hash = password::hash_password(
        &payload.password,
        state.config.argon2_memory_cost,
        state.config.argon2_time_cost,
        state.config.argon2_parallelism,
    )?;

    let user: crate::models::user::User = user_repo::create_user(
        &state.pool,
        &payload.email,
        &hash,
        payload.display_name.as_deref(),
        &payload.full_name,
        &payload.username,
        &payload.u_role,
    )
    .await?;

    tracing::info!(user_id = %user.id, "user registered");

    // let refresh = jwt::issue_token(
    //     user.id,
    //     "user",
    //     "refresh",
    //     state.config.jwt_access_expiry_secs,
    // )?;

    match email_store::issue_token(
        &state.redis,
        &payload.email,
        state.config.email_verification_expiry_secs,
        state.config.email_verify_rate_limit_secs,
    )
    .await
    {
        Ok(vtoken) => {
            if let Err(e) = email::send_verification_email(&state, &payload.email, &vtoken).await {
                tracing::warn!(error = %e, "verification email send failed");
            }
        }
        Err(e) => {
            // لا نفشل التسجيل لأن الإيميل لم يُرسل — المستخدم يستطيع طلب إعادة الإرسال
            tracing::warn!(error = %e, "verification token issue failed");
        }
    }




    let tokens = issue_tokens(&state, &user.id.as_str()).await?;
    Ok(Json(AuthResponse {
        user_id: user.id,

        tokens,
    }))
}

#[tracing::instrument(skip(state, payload), fields(email = %payload.email))]
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    validation::validate(&payload)?;

    let user = user_repo::find_by_email(&state.pool, &payload.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let ok = password::verify_password(&payload.password, &user.password_hash)?;
    if !ok {
        tracing::warn!(email = %payload.email, "login failed: bad password");
        return Err(AppError::Unauthorized);
    }

    tracing::info!(user_id = %user.id, "login success");
    let tokens = issue_tokens(&state, &user.id.as_str()).await?;
    Ok(Json(AuthResponse {
        user_id: user.id,

        tokens,
    }))
}

#[tracing::instrument(skip(state, payload))]
pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<TokenPair>, AppError> {
    let hash = token::hash_token(&payload.refresh_token);

    let row = token_repo::find_by_hash(&state.pool, &hash)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if row.revoked || row.rotated {
        tracing::warn!(
            user_id = %row.user_id,
            family_id = %row.family_id,
            "refresh token reuse detected — revoking family"
        );
        token_repo::revoke_family(&state.pool, &row.family_id.as_str()).await?;
        return Err(AppError::Unauthorized);
    }

    if row.expires_at < Utc::now() {
        tracing::debug!("refresh token expired");
        return Err(AppError::Unauthorized);
    }

    token_repo::mark_rotated(&state.pool, &row.id.as_str()).await?;
    //end ...

    let access = jwt::issue_token(
        row.user_id.clone(),
        row.u_role.as_str(),
        "access",
        state.config.jwt_access_expiry_secs,
    )?;
    let refresh = jwt::issue_token(
        row.user_id.clone(),
        row.u_role.as_str(),
        "refresh",
        state.config.jwt_access_expiry_secs,
    )?;

    let new_hash = token::hash_token(&refresh);
    let expires_at = Utc::now() + Duration::seconds(state.config.jwt_refresh_expiry_secs);

    token_repo::insert_refresh_token(
        &state.pool,
        row.user_id.clone(),
        &new_hash,
        row.family_id,
        expires_at,
    )
    .await?;

    tracing::info!(user_id = %row.user_id, "refresh rotated");
    Ok(Json(TokenPair {
        access_token: access,
        refresh_token: refresh,
        token_type: "Bearer".into(),
        expires_in: state.config.jwt_access_expiry_secs,
    }))
}

#[tracing::instrument(skip(state, payload))]
pub async fn logout(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LogoutRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let hash = token::hash_token(&payload.refresh_token);
    token_repo::revoke_by_hash(&state.pool, &hash).await?;
    tracing::info!("logout — refresh revoked");
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[tracing::instrument(skip(state, payload))]
pub async fn delete(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<DeleteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    token_repo::delete(&state.pool, &payload.user_id).await?;
    user_repo::delete(&state.pool, &payload.user_id).await?;
    tracing::info!("delete");
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[tracing::instrument(skip(state, q))]
pub async fn verify_email(
    State(state): State<Arc<AppState>>,
    Query(q): Query<VerifyEmailQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let email = email_store::consume_token(&state.redis, &q.token).await?;
    let updated = user_repo::mark_email_verified(&state.pool, &email).await?;
    if updated == 0 {
        return Err(AppError::NotFound);
    }
    tracing::info!(%email, "email verified");
    Ok(Json(serde_json::json!({ "ok": true, "email": email })))
}

async fn issue_tokens(state: &AppState, user_id: &str) -> Result<TokenPair, AppError> {
    let access = jwt::issue_token(
        user_id.to_string(),
        "user",
        "access",
        state.config.jwt_access_expiry_secs,
    )?;

    let refresh = jwt::issue_token(
        user_id.to_string(),
        "user",
        "refresh",
        state.config.jwt_access_expiry_secs,
    )?;

    let refresh_hash = token::hash_token(&refresh);

    let family_id = Uuid::now_v7().to_string();
    let expires_at = Utc::now() + Duration::seconds(state.config.jwt_refresh_expiry_secs);

    token_repo::insert_refresh_token(
        &state.pool,
        user_id.to_string(),
        &refresh_hash,
        family_id,
        expires_at,
    )
    .await?;

    Ok(TokenPair {
        access_token: access,
        refresh_token: refresh,
        token_type: "Bearer".into(),
        expires_in: state.config.jwt_access_expiry_secs,
    })
}
