use async_graphql::ErrorExtensions;
use category_service::error::AppError;

#[test]
fn display_messages() {
    assert_eq!(AppError::Config("x".into()).to_string(), "Config: x");
    assert_eq!(AppError::NotFound("c 1".into()).to_string(), "Not found: c 1");
    assert_eq!(AppError::Conflict("dup".into()).to_string(), "Conflict: dup");
    assert_eq!(AppError::Validation("v".into()).to_string(), "Validation: v");
    assert_eq!(AppError::Internal("boom".into()).to_string(), "Internal: boom");
}

#[test]
fn grpc_status_from_app_error() {
    let s: tonic::Status = AppError::Internal("boom".into()).into();
    assert_eq!(s.code(), tonic::Code::Internal);
    assert!(s.message().contains("boom"));
}

#[test]
fn graphql_extend_wraps_message() {
    let gql = AppError::Validation("bad".into()).extend();
    assert!(gql.message.contains("Validation: bad"));
}

#[test]
fn from_sqlx_error() {
    let e: AppError = sqlx::Error::RowNotFound.into();
    assert!(matches!(e, AppError::Database(_)));
}