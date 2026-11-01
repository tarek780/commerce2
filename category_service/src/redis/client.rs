use redis::aio::ConnectionManager;
use crate::error::AppError;

pub async fn create_redis_manager(url: &str) -> Result<ConnectionManager, AppError> {
    let client = redis::Client::open(url)?;
    Ok(ConnectionManager::new(client).await?)
}