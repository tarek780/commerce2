use inventory_service::error::InventoryError;
use tonic::Code;

pub fn maps_domain_errors_to_status_codes() {
    let cases: Vec<(InventoryError, Code)> = vec![
        (InventoryError::VariantNotFound, Code::NotFound),
        (InventoryError::ReservationNotFound, Code::NotFound),
        (InventoryError::DuplicateReservation, Code::AlreadyExists),
        (InventoryError::IdempotencyMismatch, Code::InvalidArgument),
        (
            InventoryError::InsufficientStock { available: 3 },
            Code::FailedPrecondition,
        ),
        (InventoryError::ReservationExpired, Code::FailedPrecondition),
        (
            InventoryError::InvalidState("x".into()),
            Code::FailedPrecondition,
        ),
    ];

    for (err, expected) in cases {
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), expected);
    }
}
