use async_trait::async_trait;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::error::AppResult;

#[async_trait]
pub trait CategoryClient: Send + Sync + 'static {
    async fn get_variant_price(&self, variant_id: Uuid) -> AppResult<BigDecimal>;
    async fn variant_exists(&self, variant_id: Uuid) -> AppResult<bool>;
}

pub struct PlaceholderCategoryClient {
    _graphql_url: String,
}

impl PlaceholderCategoryClient {
    pub fn new(cfg: AppConfig) -> Self {
        Self {
            _graphql_url: cfg.category_graphql_url,
        }
    }
}

#[async_trait]
impl CategoryClient for PlaceholderCategoryClient {
    async fn get_variant_price(&self, variant_id: Uuid) -> AppResult<BigDecimal> {
        let x = variant_id.to_string();
        tracing::warn!(x, "PlaceholderCategoryClient: using stub price");

        Ok(BigDecimal::from_str("0").unwrap())
    }

    async fn variant_exists(&self, _variant_id: Uuid) -> AppResult<bool> {
        Ok(true)
    }
}
