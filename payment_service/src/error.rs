use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tonic::Status;

#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    #[error("payment not found")]
    NotFound,
    #[error("duplicate idempotency")]
    Duplicate,
    #[error("idempotency mismatch")]
    IdempotencyMismatch,
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("gateway: {0}")]
    Gateway(String),
    #[error("webhook verification failed")]
    WebhookVerification,
    #[error("unauthorized")]
    Unauthorized,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("invalid quantity: must be > 0")]
    InvalidQuantity,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("TonicStatus : {0}")]
    TonicStatus(#[from] tonic::Status),
    #[error("env variable error: {0}")]
    VarError(#[from] std::env::VarError),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("axum extension missing: {0}")]
    AxumExtension(String),
    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] axum::http::header::InvalidHeaderValue),
    #[error("http layer error: {0}")]
    Http(#[from] axum::http::Error),
}

impl From<PaymentError> for Status {
    fn from(e: PaymentError) -> Self {
        match e {
            PaymentError::NotFound => Status::not_found("payment not found"),
            PaymentError::Duplicate => Status::already_exists("duplicate"),
            PaymentError::IdempotencyMismatch => Status::invalid_argument("idempotency mismatch"),
            PaymentError::InvalidState(s) => {
                Status::failed_precondition(format!("invalid state: {s}"))
            }
            PaymentError::Gateway(m) => Status::unavailable(format!("gateway: {m}")),
            PaymentError::WebhookVerification => {
                Status::unauthenticated("webhook verification failed")
            }
            PaymentError::Unauthorized => Status::unauthenticated("unauthorized"),
            PaymentError::BadRequest(msg) => Status::invalid_argument(msg),
            PaymentError::InvalidQuantity => {
                Status::invalid_argument("invalid quantity: must be > 0")
            }
            PaymentError::AxumExtension(msg) => Status::internal(msg),

            PaymentError::Database(err) => {
                tracing::error!(?err);
                Status::internal(err.to_string())
            }
            PaymentError::TonicStatus(s) => {
                tracing::error!(?s);
                Status::internal(s.message().to_string())
            }
            PaymentError::VarError(err) => {
                tracing::error!(?err);
                Status::internal("internal configuration error")
            }

            PaymentError::Serde(err) => {
                tracing::error!(?err);
                Status::internal("serialization error")
            }

            PaymentError::InvalidHeader(err) => {
                tracing::error!(?err);
                Status::internal("invalid header configuration")
            }
            PaymentError::Http(err) => {
                tracing::error!(?err);
                Status::internal("internal http error")
            }
        }
    }
}

impl IntoResponse for PaymentError {
    fn into_response(self) -> Response {
        let (status, client_msg) = match &self {
            PaymentError::NotFound => {
                tracing::warn!(error = ?self, "Resource not found");
                (StatusCode::NOT_FOUND, self.to_string())
            }
            PaymentError::Duplicate => {
                tracing::warn!(error = ?self, "Duplicate idempotency key");
                (StatusCode::CONFLICT, self.to_string())
            }
            PaymentError::IdempotencyMismatch => {
                tracing::warn!(error = ?self, "Idempotency key mismatch");
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            PaymentError::InvalidState(msg) => {
                tracing::warn!(error = ?self, "Invalid state transition");
                (StatusCode::BAD_REQUEST, format!("invalid state: {msg}"))
            }
            PaymentError::Gateway(msg) => {
                tracing::warn!(error = ?self, "Payment gateway failure");
                (StatusCode::BAD_GATEWAY, format!("gateway error: {msg}"))
            }
            PaymentError::WebhookVerification => {
                tracing::warn!(error = ?self, "Webhook verification failed");
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            PaymentError::InvalidQuantity => {
                tracing::warn!(error = ?self, "Invalid request parameters");
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            PaymentError::BadRequest(msg) => {
                tracing::warn!(error = ?self, "Bad request made by client");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            PaymentError::Unauthorized => {
                tracing::warn!(error = ?self, "Unauthorized access attempt");
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            PaymentError::AxumExtension(msg) => {
                tracing::warn!(error = ?self, "Missing or invalid axum extension");
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            PaymentError::InvalidHeader(err) => {
                tracing::warn!(error = ?err, "Invalid HTTP header value provided");
                (StatusCode::BAD_REQUEST, err.to_string())
            }

            PaymentError::TonicStatus(s) => {
                tracing::error!(status_code = %StatusCode::INTERNAL_SERVER_ERROR, error = ?self, "Database or Tonic status error occurred");
                (StatusCode::INTERNAL_SERVER_ERROR, s.message().to_string())
            }

            PaymentError::Database(_)
            | PaymentError::VarError(_)
            | PaymentError::Serde(_)
            | PaymentError::Http(_) => {
                tracing::error!(status_code = %StatusCode::INTERNAL_SERVER_ERROR, error = ?self, "Internal server error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": client_msg }))).into_response()
    }
}
