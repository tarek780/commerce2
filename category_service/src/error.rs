use async_graphql::ErrorExtensions;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Config: {0}")]
    Config(String),
    #[error("DB: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Redis: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Validation: {0}")]
    Validation(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("gRPC: {0}")]
    Grpc(String),
    #[error("Internal: {0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl ErrorExtensions for AppError {
    fn extend(&self) -> async_graphql::Error {
        async_graphql::Error::new(self.to_string())
    }
}

// impl From<AppError> for async_graphql::Error {
//     fn from(e: AppError) -> Self { async_graphql::Error::new(e.to_string()) }
// }

impl From<AppError> for tonic::Status {
    fn from(e: AppError) -> Self {
        tonic::Status::internal(e.to_string())
    }
}
