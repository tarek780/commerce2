use tonic::transport::Channel;
use crate::pb_inv::inventory_service_client::InventoryServiceClient;
use crate::pb_inv::{CommitRequest, ReleaseRequest, ReserveRequest};

#[derive(Clone)]
pub struct InventoryClient {
    inner: InventoryServiceClient<Channel>,
}

impl InventoryClient {
    pub fn new(channel: Channel) -> Self {
        Self { inner: InventoryServiceClient::new(channel) }
    }

    pub async fn reserve(
        &self,
        key: &str,
        variant_id: i32,
        quantity: i32,
        ttl_seconds: i64,
    ) -> Result<(String, i64), tonic::Status> {
        let mut c = self.inner.clone();
        let r = c.reserve(ReserveRequest {
            reservation_key: key.to_string(),
            variant_id,
            quantity,
            ttl_seconds,
        }).await?.into_inner();
        Ok((r.reservation_id, r.expires_at_unix))
    }

    pub async fn release(&self, reservation_id: &str) -> Result<(), tonic::Status> {
        let mut c = self.inner.clone();
        c.release(ReleaseRequest { reservation_id: reservation_id.to_string() }).await?;
        Ok(())
    }

    pub async fn commit(&self, reservation_id: &str) -> Result<(), tonic::Status> {
        let mut c = self.inner.clone();
        c.commit(CommitRequest { reservation_id: reservation_id.to_string() }).await?;
        Ok(())
    }
}