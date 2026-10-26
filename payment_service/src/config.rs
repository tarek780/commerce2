// config.rs
use crate::error::PaymentError;
#[derive(Debug, Clone)]
pub struct PaymentConfig {
    pub database_url: String,
    pub grpc_addr: String,
    pub http_addr: String,
    pub gateway: String,
    pub gateway_public_base: String,
    pub webhook_secret: String,
    pub reservation_ttl_secs: i64,
    pub expiry_scan_secs: u64,
    pub kafka_brokers: String,
    pub kafka_payment_topic: String,
}

// state.rs
// #[derive(Clone)]
// pub struct AppState {
//     pub db: sqlx::PgPool,
//     pub gateway: std::sync::Arc<dyn crate::gateway::PaymentGateway>,
//     pub config: std::sync::Arc<PaymentConfig>,
// }
impl PaymentConfig {
    pub async fn from_env() -> Result<Self, PaymentError> {
        let x = Self {
            database_url: get("DATABASE_URL").await?,
            grpc_addr: get("GRPC_ADDR").await?,
            kafka_brokers: get("KAFKA_BROKERS").await?,
            kafka_payment_topic: get("KAFKA_PAYMENT_TOPIC").await?,
            expiry_scan_secs: get("EXPIRY_SCAN_SECS").await?.parse().unwrap(),
            gateway: get("PAYMENT_GATEWAY").await?,
            gateway_public_base: get("GATEWAY_PUBLIC_BASE").await?,
            http_addr: get("HTTP_ADDR").await?,
            reservation_ttl_secs: get("RESERVATION_TTL_SECS").await?.parse().unwrap(),
            webhook_secret: get("WEBHOOK_SECRET").await?,
        };
        Ok(x)
    }
}

async fn get(key: &str) -> Result<String, PaymentError> {
    let key = std::env::var(key)?;
    Ok(key)
}
