use crate::config::AppConfig;
use crate::error::AppError;
use crate::helpers::idempotency::{IdempotencyStore, RedisIdempotencyStore};
use std::sync::Arc;
#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<AppConfig>,
    pub cart: Arc<dyn crate::clients::cart::CartClient>,
    pub category: Arc<dyn crate::clients::category::CategoryClient>,
    pub order: Arc<dyn crate::clients::order::OrderClient>,
    pub payment: Arc<dyn crate::clients::payment::PaymentClient>,
    pub idempotency: Arc<dyn IdempotencyStore>,
}

impl AppState {
    pub async fn new(cfg: &AppConfig) -> Result<Self, AppError> {
        let http = reqwest::Client::builder()
            .timeout(cfg.upstream_timeout)
            .build()
            .map_err(|e| AppError::Internal(format!("http: {e}")))?;

        let redis_client = redis::Client::open(cfg.redis_url.clone())
            .map_err(|e| AppError::Internal(format!("redis: {e}")))?;
        let redis = redis::aio::ConnectionManager::new(redis_client)
            .await
            .map_err(|e| AppError::Internal(format!("redis conn: {e}")))?;

        let idempotency = Arc::new(RedisIdempotencyStore::new(redis, cfg.idempotency_ttl_secs));

        Ok(Self {
            cfg: Arc::new(cfg.clone()),
            cart: Arc::new(crate::clients::cart::HttpCartClient::new(
                http.clone(),
                cfg.clone(),
            )),
            category: Arc::new(crate::clients::category::PlaceholderCategoryClient::new(
                cfg.clone(),
            )),
            order: Arc::new(
                crate::clients::order::GrpcOrderClient::new(&cfg.order_grpc_url).await?,
            ),
            payment: Arc::new(
                crate::clients::payment::GrpcPaymentClient::new(&cfg.payment_grpc_url).await?,
            ),
            idempotency,
        })
    }

    // for tests
    pub fn for_tests(
        cfg: AppConfig,
        cart: Arc<dyn crate::clients::cart::CartClient>,
        category: Arc<dyn crate::clients::category::CategoryClient>,
        order: Arc<dyn crate::clients::order::OrderClient>,
        payment: Arc<dyn crate::clients::payment::PaymentClient>,
        idempotency: Arc<dyn IdempotencyStore>,
    ) -> Self {
        Self {
            cfg: Arc::new(cfg),
            cart,
            category,
            order,
            payment,
            idempotency,
        }
    }
}
