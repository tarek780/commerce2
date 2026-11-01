use crate::problem::Problem;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Config: {0}")]
    Config(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation: {0}")]
    Validation(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Cart error: {0}")]
    Cart(String),

    #[error(
        "Insufficient stock, variant {variant_id}: requested {requested}, available {available}"
    )]
    InsufficientStock {
        variant_id: i64,
        requested: i32,
        available: i32,
    },

    #[error("Payment required: {0}")]
    PaymentRequired(String),

    #[error("Upstream error: {0}")]
    Upstream(String),

    #[error("Idempotency conflict")]
    IdempotencyConflict,

    #[error("Internal: {0}")]
    Internal(String),

    #[error("invalid quantity: must be > 0")]
    InvalidQuantity,

    #[error("bad request: {0}")]
    BadRequest(String),

    // --- Automatic Converters via #[from] ---
    #[error("uuid error: {0}")]
    Uuid(#[from] uuid::Error),

    #[error("env variable error: {0}")]
    VarError(#[from] std::env::VarError),

    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("axum extension missing: {0}")]
    AxumExtension(String),

    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] axum::http::header::InvalidHeaderValue),

    #[error("JsonWebToken error: {0}")]
    JsonToken(#[from] jsonwebtoken::errors::Error),

    #[error("http layer error: {0}")]
    Http(#[from] axum::http::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, detail) = match &self {
            AppError::Unauthorized(_) => {
                (StatusCode::UNAUTHORIZED, "unauthorized", self.to_string())
            }
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not-found", self.to_string()),
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, "validation", self.to_string()),
            AppError::Conflict(_) | AppError::IdempotencyConflict => {
                (StatusCode::CONFLICT, "conflict", self.to_string())
            }
            AppError::Cart(_) => (StatusCode::BAD_REQUEST, "cart-error", self.to_string()),
            AppError::InvalidQuantity => (
                StatusCode::BAD_REQUEST,
                "invalid-quantity",
                self.to_string(),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad-request", msg.clone()),
            AppError::AxumExtension(msg) => {
                (StatusCode::BAD_REQUEST, "missing-extension", msg.clone())
            }
            AppError::InvalidHeader(err) => {
                (StatusCode::BAD_REQUEST, "invalid-header", err.to_string())
            }
            AppError::JsonToken(err) => {
                (StatusCode::BAD_REQUEST, "invalid-token", err.to_string())
            }

            AppError::InsufficientStock { .. } => (
                StatusCode::PAYMENT_REQUIRED,
                "insufficient-stock",
                self.to_string(),
            ),
            AppError::PaymentRequired(_) => (
                StatusCode::PAYMENT_REQUIRED,
                "payment-required",
                self.to_string(),
            ),

            AppError::Upstream(_) => (StatusCode::BAD_GATEWAY, "upstream-error", self.to_string()),

            AppError::Config(_)
            | AppError::Internal(_)
           
            | AppError::Uuid(_)
            | AppError::VarError(_)
            | AppError::Redis(_)
            | AppError::Serde(_)
            | AppError::Io(_)
            | AppError::Http(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal-error",
                "An unexpected internal error occurred.".to_string(),
            ),
        };

        if status.is_server_error() {
            tracing::error!(error = ?self, %status, code, "Internal server sub-system failure");
        } else {
            tracing::warn!(error = ?self, %status, code, "Request validation or client state rejection");
        }

        let problem = Problem::new(status, code, &detail);
        problem.into_response()
    }
}
