use async_trait::async_trait;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use storefront_service::clients::category::CategoryClient;
use storefront_service::error::{AppError, AppResult};

#[derive(Default)]
pub struct MockCategoryClient {
    pub prices: Arc<Mutex<HashMap<i64, BigDecimal>>>,
    pub fail_on: Arc<Mutex<Option<i64>>>,
}

impl MockCategoryClient {
    pub fn new() -> Self { Self::default() }

    pub async fn with_prices(prices: Vec<(i64, &str)>) -> Self {
        let s = Self::new();
        let mut m = s.prices.lock().await;
        for (id, p) in prices {
            m.insert(id, BigDecimal::from_str(p).unwrap());
        }
        s
    }

    pub async fn fail_for(&self, variant_id: i64) {
        *self.fail_on.lock().await = Some(variant_id);
    }
}

#[async_trait]
impl CategoryClient for MockCategoryClient {
    async fn get_variant_price(&self, variant_id: i64) -> AppResult<BigDecimal> {
        if *self.fail_on.lock().await == Some(variant_id) {
            return Err(AppError::Upstream("category unavailable".into()));
        }
        let m = self.prices.lock().await;
        Ok(m.get(&variant_id).cloned().unwrap_or_else(|| BigDecimal::from(0)))
    }
    async fn variant_exists(&self, _variant_id: i64) -> AppResult<bool> { Ok(true) }
}