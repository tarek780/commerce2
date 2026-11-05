use async_trait::async_trait;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use storefront_service::clients::category::CategoryClient;
use storefront_service::error::{AppError, AppResult};

#[derive(Default)]
pub struct MockCategoryClient {
    pub prices: Arc<Mutex<HashMap<Uuid, BigDecimal>>>,
    pub fail_on: Arc<Mutex<Option<Uuid>>>,
}

impl MockCategoryClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn with_prices(&self, prices: Vec<(Uuid, &str)>) -> Self {
        let s = Self::new();
        let mut m = s.prices.lock().await;
        for (id, p) in prices {
            m.insert(id, BigDecimal::from_str(p).unwrap());
        }
        drop(m);
        s
    }

    pub async fn fail_for(&self, variant_id: Uuid) {
        *self.fail_on.lock().await = Some(variant_id);
    }
}

#[async_trait]
impl CategoryClient for MockCategoryClient {
    async fn get_variant_price(&self, variant_id: Uuid) -> AppResult<BigDecimal> {
        if *self.fail_on.lock().await == Some(variant_id) {
            return Err(AppError::Upstream("category unavailable".into()));
        }
        let m = self.prices.lock().await;
        Ok(m.get(&variant_id)
            .cloned()
            .unwrap_or_else(|| BigDecimal::from(0)))
    }
    async fn variant_exists(&self, _variant_id: Uuid) -> AppResult<bool> {
        Ok(true)
    }
}
