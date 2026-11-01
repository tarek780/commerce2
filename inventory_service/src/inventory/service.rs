use chrono::Utc;
use std::sync::Arc;
use tokio::sync::broadcast;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use async_stream::try_stream;

use crate::inventory::repo::InventoryRepo;
use crate::pb::inventory_service_server::InventoryService;
use crate::pb::*;

pub struct InventorySvc {
    pub repo: Arc<InventoryRepo>,
    pub stock_tx: broadcast::Sender<StockUpdate>,
}

impl InventorySvc {
    pub fn new(repo: Arc<InventoryRepo>, stock_tx: broadcast::Sender<StockUpdate>) -> Self {
        Self { repo, stock_tx }
    }

    fn emit(&self, variant_id: i32, available: i32, reserved: i32, event_type: &str) {
        let _ = self.stock_tx.send(StockUpdate {
            variant_id,
            available_quantity: available,
            reserved_quantity: reserved,
            event_type: event_type.to_string(),
            updated_at_unix: Utc::now().timestamp(),
        });
    }
}

#[tonic::async_trait]
impl InventoryService for InventorySvc {
    type WatchStockStream = std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<StockUpdate, Status>> + Send + 'static>,
    >;

    async fn reserve(
        &self,
        req: Request<ReserveRequest>,
    ) -> Result<Response<ReserveResponse>, Status> {
        let r = req.into_inner();
        let res = self
            .repo
            .reserve(&r.reservation_key, r.variant_id, r.quantity, r.ttl_seconds)
            .await?;

        if let Ok((stock, reserved)) = self.repo.get_stock(r.variant_id).await {
            self.emit(r.variant_id, stock - reserved, reserved, "reserved");
        }

        Ok(Response::new(ReserveResponse {
            reservation_id: res.id.to_string(),
            variant_id: res.variant_id,
            quantity: res.quantity,
            expires_at_unix: res.expires_at.timestamp(),
        }))
    }

    async fn release(
        &self,
        req: Request<ReleaseRequest>,
    ) -> Result<Response<ReleaseResponse>, Status> {
        let r = req.into_inner();
        let id =
            Uuid::parse_str(&r.reservation_id).map_err(|_| Status::invalid_argument("bad uuid"))?;
        self.repo.release(id).await?;
        Ok(Response::new(ReleaseResponse { released: true }))
    }

    async fn commit(
        &self,
        req: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        let r = req.into_inner();
        let id =
            Uuid::parse_str(&r.reservation_id).map_err(|_| Status::invalid_argument("bad uuid"))?;
        self.repo.commit(id).await?;
        Ok(Response::new(CommitResponse { committed: true }))
    }

    async fn get_stock(
        &self,
        req: Request<GetStockRequest>,
    ) -> Result<Response<GetStockResponse>, Status> {
        let r = req.into_inner();
        let (stock, reserved) = self.repo.get_stock(r.variant_id).await?;
        Ok(Response::new(GetStockResponse {
            stock_quantity: stock,
            reserved_quantity: reserved,
            available_quantity: stock - reserved,
        }))
    }

    async fn adjust_stock(
        &self,
        req: Request<AdjustStockRequest>,
    ) -> Result<Response<AdjustStockResponse>, Status> {
        let r = req.into_inner();
        let new_stock = self
            .repo
            .adjust(r.variant_id, r.delta, &r.reference_id)
            .await?;
        if let Ok((_, reserved)) = self.repo.get_stock(r.variant_id).await {
            self.emit(r.variant_id, new_stock - reserved, reserved, "adjusted");
        }
        Ok(Response::new(AdjustStockResponse {
            new_stock_quantity: new_stock,
        }))
    }

    async fn watch_stock(
        &self,
        req: Request<WatchStockRequest>,
    ) -> Result<Response<Self::WatchStockStream>, Status> {
        let r = req.into_inner();
        let mut rx = self.stock_tx.subscribe();

        let watched = if r.variant_ids.is_empty() {
            None
        } else {
            Some(r.variant_ids)
        };
        let threshold = r.low_stock_threshold.max(1);

        let out_stream = try_stream! {
            loop {
                match rx.recv().await {
                    Ok(upd) => {
                        if let Some(ref ids) = watched {
                            if !ids.contains(&upd.variant_id) {
                                continue;
                            }
                        }
                        let emit = upd.available_quantity <= threshold
                            || matches!(upd.event_type.as_str(), "reserved" | "released" | "committed" | "adjusted");

                        if emit {
                            yield upd;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "watcher lagged");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        };

        Ok(Response::new(Box::pin(out_stream) as Self::WatchStockStream))
    }
}
