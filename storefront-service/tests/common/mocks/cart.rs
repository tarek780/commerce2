use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use storefront_service::clients::cart::CartClient;
use storefront_service::dto::cart::CartResponse;
use storefront_service::error::{AppError, AppResult};

#[derive(Default)]
pub struct MockCartClient {
    pub items: Arc<Mutex<Vec<storefront_service::dto::cart::CartItem>>>,
    pub fail_clear: Arc<Mutex<bool>>,
    pub fail_get: Arc<Mutex<Option<AppError>>>,
    pub clear_calls: Arc<Mutex<usize>>,
}

impl MockCartClient {
    pub fn new() -> Self { Self::default() }

    pub async fn with_items(items: Vec<storefront_service::dto::cart::CartItem>) -> Self {
        let s = Self::new();
        *s.items.lock().await = items;
        s
    }

    pub async fn set_fail_clear(&self, val: bool) {
        *self.fail_clear.lock().await = val;
    }
}

#[async_trait]
impl CartClient for MockCartClient {
    async fn get_cart(&self, _user_id: Uuid) -> AppResult<CartResponse> {
        if let Some(e) = self.fail_get.lock().await.take() {
            return Err(e);
        }
        let items = self.items.lock().await.clone();
        if items.is_empty() {
            return Err(AppError::Cart("empty cart".into()));
        }
        Ok(CartResponse { items, updated_at: chrono::Utc::now() })
    }

    async fn clear_cart(&self, _user_id: Uuid) -> AppResult<()> {
        *self.clear_calls.lock().await += 1;
        if *self.fail_clear.lock().await {
            return Err(AppError::Upstream("cart clear failed".into()));
        }
        self.items.lock().await.clear();
        Ok(())
    }
}