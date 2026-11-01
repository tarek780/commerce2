use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::dto::cart::CartResponse;
use crate::error::{AppError, AppResult};

#[async_trait]
pub trait CartClient: Send + Sync + 'static {
    /// GET /api/cart مع x-user-id
    async fn get_cart(&self, user_id: Uuid) -> AppResult<CartResponse>;

    async fn clear_cart(&self, user_id: Uuid) -> AppResult<()>;
}

pub struct HttpCartClient {
    http: reqwest::Client,
    base: String,
    clear_endpoint: String,
}

impl HttpCartClient {
    pub fn new(http: reqwest::Client, cfg: AppConfig) -> Self {
        Self { http, base: cfg.cart_base_url, clear_endpoint: cfg.cart_clear_endpoint }
    }
}

#[async_trait]
impl CartClient for HttpCartClient {
    async fn get_cart(&self, user_id: Uuid) -> AppResult<CartResponse> {
        let url = format!("{}/api/cart", self.base.trim_end_matches('/'));
        let resp = self
            .http
            .get(&url)
            .header("x-user-id", user_id.to_string())
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("cart get: {e}")))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Cart("cart not found".into()));
        }
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!("cart get {status}: {body}")));
        }
        resp.json::<CartResponse>()
            .await
            .map_err(|e| AppError::Upstream(format!("cart decode: {e}")))
    }

    async fn clear_cart(&self, user_id: Uuid) -> AppResult<()> {
        let url = format!(
            "{}{}",
            self.base.trim_end_matches('/'),
            self.clear_endpoint
        );
        let resp = self
            .http
            .delete(&url)
            .header("x-user-id", user_id.to_string())
            .send()
            .await
            .map_err(|e| AppError::Upstream(format!("cart clear: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(AppError::Upstream(format!("cart clear failed: {status}")));
        }
        Ok(())
    }
}

/// for tests
pub struct MockCartClient {
    pub items: Arc<tokio::sync::Mutex<Vec<crate::dto::cart::CartItem>>>,
}

#[async_trait]
impl CartClient for MockCartClient {
    async fn get_cart(&self, _user_id: Uuid) -> AppResult<CartResponse> {
        let items = self.items.lock().await.clone();
        if items.is_empty() {
            return Err(AppError::Cart("empty cart".into()));
        }
        Ok(CartResponse {
            items,
            updated_at: chrono::Utc::now(),
        })
    }
    async fn clear_cart(&self, _user_id: Uuid) -> AppResult<()> {
        self.items.lock().await.clear();
        Ok(())
    }
}