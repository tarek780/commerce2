use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub redis_stream_key: String,
    pub redis_consumer_group: String,
    pub redis_consumer_name: String,
    pub inventory_grpc_url: String,
    pub bind_addr: String,
    pub index_products: String,
    pub index_categories: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            database_url: env("DATABASE_URL")?,
            redis_url: env("REDIS_URL")?,
            redis_stream_key: env("REDIS_STREAM_KEY")?,
            redis_consumer_group: env("REDIS_CONSUMER_GROUP")?,
            redis_consumer_name: std::env::var("REDIS_CONSUMER_NAME")
                .unwrap_or_else(|_| "worker-1".into()),
            inventory_grpc_url: env("INVENTORY_GRPC_URL")?,
            bind_addr: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:8084".into()),
            index_products: std::env::var("REDIS_INDEX_PRODUCTS")
                .unwrap_or_else(|_| "catalog:products:idx".into()),
            index_categories: std::env::var("REDIS_INDEX_CATEGORIES")
                .unwrap_or_else(|_| "catalog:categories:idx".into()),
        })
    }
}

fn env(k: &str) -> Result<String, AppError> {
    std::env::var(k).map_err(|_| AppError::Config(format!("missing {k}")))
}