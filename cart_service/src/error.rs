use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum CartError {
    #[error("cart not found")]
    NotFound,

    #[error("invalid quantity: must be > 0")]
    InvalidQuantity,

    #[error("unauthorized")]
    Unauthorized,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("database migration error: {0}")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),

    #[error("env variable error: {0}")]
    VarError(#[from] std::env::VarError),

    #[error("JsonWebToken error: {0}")]
    JsonWebToken(#[from] jsonwebtoken::errors::Error),

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

    #[error("http layer error: {0}")]
    Http(#[from] axum::http::Error),
}

impl IntoResponse for CartError {
    fn into_response(self) -> Response {
        let (status, client_msg) = match &self {
            CartError::NotFound => {
                tracing::warn!(error = ?self, "Resource not found");
                (StatusCode::NOT_FOUND, self.to_string())
            }
            CartError::InvalidQuantity => {
                tracing::warn!(error = ?self, "Invalid request parameters");
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            CartError::BadRequest(msg) => {
                tracing::warn!(error = ?self, "Bad request made by client");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            CartError::Unauthorized => {
                tracing::warn!(error = ?self, "Unauthorized access attempt");
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            CartError::AxumExtension(msg) => {
                tracing::warn!(error = ?self, "Missing or invalid axum extension");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            CartError::InvalidHeader(err) => {
                tracing::warn!(error = ?err, "Invalid HTTP header value provided");
                (StatusCode::BAD_REQUEST, err.to_string())
            }

            CartError::Database(err) => {
                tracing::warn!(error = ?err, "Database operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }

            CartError::DatabaseMigration(err) => {
                tracing::error!(error = ?err, "Database migration failed");
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }

            CartError::VarError(err) => {
                tracing::error!(error = ?err, "Environment variable error");
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            CartError::JsonWebToken(error) => {
                tracing::error!(error = ?error, "JWT processing error");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
            CartError::Redis(error) => {
                tracing::error!(error = ?error, "Redis operation failed");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
            CartError::Serde(error) => {
                tracing::error!(error = ?error, "Serialization/Deserialization error");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
            CartError::Io(error) => {
                tracing::error!(error = ?error, "I/O error");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
            CartError::Http(error) => {
                tracing::error!(error = ?error, "HTTP error");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
            }
        };
        (status, Json(json!({ "error": client_msg }))).into_response()
    }
}
