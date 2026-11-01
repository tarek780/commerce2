use tonic::Status;

#[derive(Debug, thiserror::Error)]
pub enum InventoryError {
    #[error("variant not found")]
    VariantNotFound,
    #[error("reservation not found")]
    ReservationNotFound,
    #[error("duplicate reservation")]
    DuplicateReservation,
    #[error("idempotency mismatch")]
    IdempotencyMismatch,
    #[error("insufficient stock, available: {available}")]
    InsufficientStock { available: i32 },
    #[error("reservation expired")]
    ReservationExpired,
    #[error("invalid state: {0}")]
    InvalidState(String),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}

impl From<InventoryError> for Status {
    fn from(e: InventoryError) -> Self {
        match e {
            InventoryError::VariantNotFound => Status::not_found("variant not found"),
            InventoryError::ReservationNotFound => Status::not_found("reservation not found"),
            InventoryError::DuplicateReservation => {
                Status::already_exists("duplicate reservation_key")
            }
            InventoryError::IdempotencyMismatch => {
                Status::invalid_argument("reservation_key exists with different params")
            }
            InventoryError::InsufficientStock { available } => {
                Status::failed_precondition(format!("insufficient stock, available: {available}"))
            }
            InventoryError::ReservationExpired => {
                Status::failed_precondition("reservation expired")
            }
            InventoryError::InvalidState(s) => {
                Status::failed_precondition(format!("invalid state: {s}"))
            }
            InventoryError::Database(e) => {
                tracing::error!(?e, "database error");
                Status::internal("database error")
            }
        }
    }
}
