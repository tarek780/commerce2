use tonic::Status;

#[derive(Debug, thiserror::Error)]
pub enum OrderError {
    #[error("order not found")]
    NotFound,
    #[error("duplicate order (idempotency)")]
    Duplicate,
    #[error("idempotency mismatch")]
    IdempotencyMismatch,
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("inventory error: {0}")]
    Inventory(String),
    #[error("payment error: {0}")]
    Payment(String),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization: {0}")]
    Serde(#[from] serde_json::Error),
}

impl From<OrderError> for Status {
    fn from(e: OrderError) -> Self {
        match e {
            OrderError::NotFound => Status::not_found("order not found"),
            OrderError::Duplicate => Status::already_exists("duplicate order"),
            OrderError::IdempotencyMismatch => Status::invalid_argument("idempotency mismatch"),
            OrderError::InvalidState(s) => Status::failed_precondition(format!("invalid state: {s}")),
            OrderError::Inventory(m) => Status::failed_precondition(format!("inventory: {m}")),
            OrderError::Payment(m) => Status::failed_precondition(format!("payment: {m}")),
            OrderError::Database(e) => {
                tracing::error!(?e, "db error");
                Status::internal("database error")
            }
            OrderError::Serde(e) => {
                tracing::error!(?e, "serde error");
                Status::internal("internal error")
            }
        }
    }
}