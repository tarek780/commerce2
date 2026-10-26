
use tonic::transport::Channel;
use crate::pb_inv::inventory_service_client::InventoryServiceClient;
use crate::pb_inv::{AdjustStockRequest, ReleaseRequest, ReserveRequest};
use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct InventoryClient {
    inner: InventoryServiceClient<Channel>,
}

impl InventoryClient {
    pub fn new(channel: Channel) -> Self {
        Self { inner: InventoryServiceClient::new(channel) }
    }

    pub async fn ensure_sku(&self, variant_id: i32) -> AppResult<()> {
        let mut c = self.inner.clone();
        // AdjustStock +0 = idempotent ensure
        c.adjust_stock(AdjustStockRequest {
            variant_id,
            delta: 0,
            reference_id: "ensure_sku".into(),
        }).await.map_err(|e| AppError::Grpc(e.to_string()))?;
        Ok(())
    }

    pub async fn release(&self, reservation_id: &str) -> AppResult<()> {
        let mut c = self.inner.clone();
        c.release(ReleaseRequest { reservation_id: reservation_id.into() })
            .await.map_err(|e| AppError::Grpc(e.to_string()))?;
        Ok(())
    }

    pub async fn reserve(&self, key: &str, variant_id: i32, quantity: i32, ttl: i64) -> AppResult<String> {
        let mut c = self.inner.clone();
        let r = c.reserve(ReserveRequest {
            reservation_key: key.into(),
            variant_id,
            quantity,
            ttl_seconds: ttl,
        }).await.map_err(|e| AppError::Grpc(e.to_string()))?.into_inner();
        Ok(r.reservation_id)
    }
}