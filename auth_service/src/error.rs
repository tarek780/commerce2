use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("VarError : {0}")]
    VarError(#[from] std::env::VarError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("password hashing error")]
    PasswordHash(#[from] argon2::Error),

    #[error("jwt token error")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("email delivery error")]
    Email(#[from] lettre::error::Error),

    #[error("smtp transport error")]
    Smtp(#[from] lettre::transport::smtp::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        AppError::Validation(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden".to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            AppError::VarError(e) => {
                tracing::error!("VarError : {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error ".to_string(),
                )
            }

            AppError::Database(e) => {
                tracing::error!("database connection or query error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }

            AppError::PasswordHash(e) => {
                tracing::error!("crypto/argon2 error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }

            AppError::Jwt(e) => {
                tracing::warn!("jwt validation failure: {:?}", e);
                (
                    StatusCode::UNAUTHORIZED,
                    "invalid or expired token".to_string(),
                )
            }

            AppError::Email(e) => {
                tracing::error!("failed to build email: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
            AppError::Smtp(e) => {
                tracing::error!("smtp relay failure: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }

            AppError::Internal(m) => {
                tracing::error!("internal system application error: {}", m);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": msg }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
