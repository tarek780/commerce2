// use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OrderConfig {
    pub database_url: String,
    pub bind_addr: String,
    pub inventory_grpc_url: String,
    pub payment_grpc_url: String,
    pub kafka_brokers: String,
    pub kafka_payment_topic: String,
    pub kafka_payment_group: String,
    pub order_ttl_secs: i64,
    pub expiry_scan_secs: u64,
}

impl OrderConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")?,
            bind_addr: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:50053".into()),
            inventory_grpc_url: std::env::var("INVENTORY_GRPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:50051".into()),
            payment_grpc_url: std::env::var("PAYMENT_GRPC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:50052".into()),
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".into()),
            kafka_payment_topic: std::env::var("KAFKA_PAYMENT_TOPIC")
                .unwrap_or_else(|_| "payment.events".into()),
            kafka_payment_group: std::env::var("KAFKA_ORDER_PAYMENT_GROUP")
                .unwrap_or_else(|_| "order-service".into()),
            order_ttl_secs: std::env::var("ORDER_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(900),
            expiry_scan_secs: std::env::var("ORDER_EXPIRY_SCAN_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        })
    }
    // pub fn order_ttl(&self) -> Duration {
    //     Duration::from_secs(self.order_ttl_secs as u64)
    // }
}
