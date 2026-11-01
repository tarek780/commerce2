use std::time::Duration;
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub http_addr: String,
    pub currency: String,
    pub cart_base_url: String,
    pub cart_clear_endpoint: String,
    pub category_graphql_url: String,
    pub order_grpc_url: String,
    pub payment_grpc_url: String,
    pub inventory_grpc_url: String,
    pub redis_url: String,
    pub idempotency_ttl_secs: u64,
    pub idempotency_required: bool,
    pub upstream_timeout: Duration,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        Ok(Self {
            http_addr: env_or("HTTP_ADDR", "0.0.0.0:8080"),
            currency: env_or("CURRENCY", "USD"),
            cart_base_url: env("CART_BASE_URL")?,
            cart_clear_endpoint: env_or("CART_CLEAR_ENDPOINT", "/api/cart"),
            category_graphql_url: env("CATEGORY_GRAPHQL_URL")?,
            order_grpc_url: env("ORDER_GRPC_URL")?,
            payment_grpc_url: env("PAYMENT_GRPC_URL")?,
            inventory_grpc_url: env("INVENTORY_GRPC_URL")?,
            redis_url: env("REDIS_URL")?,
            idempotency_ttl_secs: env_parse("IDEMPOTENCY_TTL_SECS", 86400),
            idempotency_required: env_parse_bool("IDEMPOTENCY_REQUIRED", true),
            upstream_timeout: Duration::from_secs(
                env_parse("UPSTREAM_TIMEOUT_SECS", 10),
            ),
        })
    }
}

fn env(k: &str) -> Result<String, AppError> {
    std::env::var(k).map_err(|_| AppError::Config(format!("missing {k}")))
}

fn env_or(k: &str, d: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| d.to_string())
}

fn env_parse<T: std::str::FromStr>(k: &str, d: T) -> T {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn env_parse_bool(k: &str, d: bool) -> bool {
    std::env::var(k).ok().map(|v| matches!(v.as_str(), "1" | "true" | "yes")).unwrap_or(d)
}