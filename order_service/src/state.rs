use std::sync::Arc;
use sqlx::PgPool;
use tonic::transport::Channel;

use crate::clients::inventory::InventoryClient;
use crate::clients::payment::PaymentClient;
use crate::config::OrderConfig;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub inventory: Arc<InventoryClient>,
    pub payment: Arc<PaymentClient>,
    pub config: Arc<OrderConfig>,
}

impl AppState {
    pub async fn new(cfg: OrderConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&cfg.database_url).await?;

        let inv_channel = Channel::from_shared(cfg.inventory_grpc_url.clone())?
            .connect().await?;
        let pay_channel = Channel::from_shared(cfg.payment_grpc_url.clone())?
            .connect().await?;

        Ok(Self {
            db,
            inventory: Arc::new(InventoryClient::new(inv_channel)),
            payment: Arc::new(PaymentClient::new(pay_channel)),
            config: Arc::new(cfg),
        })
    }
}