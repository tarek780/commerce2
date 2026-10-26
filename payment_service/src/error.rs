use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tonic::Status;

// ==========================================
// تعريف الـ Enum الموحد لجميع الأخطاء (PaymentError)
// ==========================================
#[derive(Debug, thiserror::Error)]
pub enum PaymentError {
    // --- أخطاء العميل المشتركة (Client Errors) ---
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

    // --- أخطاء قواعد البيانات والكاش والبيئة ---
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("TonicStatus : {0}")]
    TonicStatus(#[from] tonic::Status),
    #[error("env variable error: {0}")]
    VarError(#[from] std::env::VarError),
    // #[error("JsonWebToken error: {0}")]
    // JsonWebToken(#[from] jsonwebtoken::errors::Error),

    // --- أخطاء التحويل والمعالجة ---
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    // --- أخطاء نظام التشغيل والشبكة والسيرفر ---
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    // --- أخطاء Axum و HTTP ---
    #[error("axum extension missing: {0}")]
    AxumExtension(String),
    #[error("invalid header value: {0}")]
    InvalidHeader(#[from] axum::http::header::InvalidHeaderValue),
    #[error("http layer error: {0}")]
    Http(#[from] axum::http::Error),
}

// ==========================================
// 1. دالة معالجة وتحويل الأخطاء لنظام gRPC (Tonic Status)
// ==========================================

impl From<PaymentError> for Status {
    fn from(e:PaymentError ) -> Self {
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

            // أخطاء السيرفر الداخلية: نسجلها في اللوج ونخفيها عن العميل الخارجي لـ gRPC
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
            // PaymentError::JsonWebToken(err) => {
            //     tracing::error!(?err);
            //     Status::internal("authentication token error")
            // }
            PaymentError::Serde(err) => {
                tracing::error!(?err);
                Status::internal("serialization error")
            }
            PaymentError::Io(err) => {
                tracing::error!(?err);
                Status::internal("internal i/o error")
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

// ==========================================
// 2. دالة معالجة وتحويل الأخطاء لنظام الويب (Axum HTTP Response)
// ==========================================
impl IntoResponse for PaymentError {
    fn into_response(self) -> Response {
        let (status, client_msg) = match &self {
            // أخطاء العميل (Client Errors) - نسجلها بـ warn لسهولة التتبع ونرسل التفاصيل واضحة للـ JSON
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

            PaymentError::TonicStatus(s)=> {
                tracing::error!(status_code = %StatusCode::INTERNAL_SERVER_ERROR, error = ?self, "Database or Tonic status error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                   s.message().to_string(),
                )

            }

            // أخطاء النظام الداخلية (Server Errors) - نسجلها بمستوى error ونرجع رسالة موحدة غير حساسة
            PaymentError::Database(_)
            
            | PaymentError::VarError(_)
            // | PaymentError::JsonWebToken(_)
            | PaymentError::Serde(_)
            | PaymentError::Io(_)
            | PaymentError::Http(_) => {
                tracing::error!(status_code = %StatusCode::INTERNAL_SERVER_ERROR, error = ?self, "Internal server error occurred");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        // إرجاع الـ JSON الموحد مع التوافق التام لـ Axum
        (status, Json(json!({ "error": client_msg }))).into_response()
    }
}
