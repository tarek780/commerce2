use redis::aio::ConnectionManager;
use sqlx::PgPool;
use tonic::transport::Channel;

use crate::config::AppConfig;
use crate::db::pool::create_pool;
use crate::error::AppError;
use crate::grpc::inventory::InventoryClient;
use crate::redis::client::create_redis_manager;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub inventory: InventoryClient,
    pub cfg: AppConfig,
}

impl AppState {
    pub async fn new(cfg: &AppConfig) -> Result<Self, AppError> {
        let db = create_pool(&cfg.database_url).await?;
        let redis = create_redis_manager(&cfg.redis_url).await?;

        let channel = Channel::from_shared(cfg.inventory_grpc_url.clone())
            .map_err(|e| AppError::Grpc(e.to_string()))?
            .connect()
            .await
            .map_err(|e| AppError::Grpc(e.to_string()))?;
        let inventory = InventoryClient::new(channel);

        Ok(Self {
            db,
            redis,
            inventory,
            cfg: cfg.clone(),
        })
    }
}
