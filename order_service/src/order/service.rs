use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::error::OrderError;
use crate::order::orchestrator;
use crate::order::repo::{OrderItemRow, OrderRepo, OrderRow};
use crate::pb_order::order_service_server::OrderService;
use crate::pb_order::*;
use crate::state::AppState;

pub struct OrderSvc {
    pub state: Arc<AppState>,
}

impl OrderSvc {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn to_proto(row: OrderRow) -> OrderResponse {
        OrderResponse {
            id: row.id.to_string(),
            customer_id: row.customer_id,
            status: row.status,
            total_amount: row.total_amount.to_string(),
            currency: row.currency,
            reservation_id: row.reservation_id.unwrap_or_default(),
            payment_id: row.payment_id.unwrap_or_default(),
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
            failure_reason: row.failure_reason,
        }
    }
}

#[tonic::async_trait]
impl OrderService for OrderSvc {
    async fn create_order(
        &self,
        req: Request<CreateOrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let r = req.into_inner();

        let items = r
            .items
            .into_iter()
            .map(|i| {
                let price = BigDecimal::from_str(&i.unit_price)
                    .map_err(|_| Status::invalid_argument("bad unit_price"))?;
                Ok(orchestrator::NewItem {
                    variant_id: i.variant_id,
                    quantity: i.quantity,
                    unit_price: price,
                })
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let key = if r.idempotency_key.is_empty() {
            Uuid::now_v7().to_string()
        } else {
            r.idempotency_key.clone()
        };

        let row = orchestrator::create_and_orchestrate(
            &self.state,
            &r.customer_id,
            &r.currency,
            items,
            &key,
        )
        .await
        .map_err(Status::from)?;

        Ok(Response::new(Self::to_proto(row)))
    }

    async fn get_order(
        &self,
        req: Request<GetOrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let id = &req.into_inner().order_id;
        let row = OrderRepo::get(&self.state.db, id.as_str()).await?;
        Ok(Response::new(Self::to_proto(row)))
    }

    async fn cancel_order(
        &self,
        req: Request<CancelOrderRequest>,
    ) -> Result<Response<OrderResponse>, Status> {
        let r = req.into_inner();

        let row = orchestrator::cancel(&self.state, &r.order_id.as_str(), &r.reason).await?;
        Ok(Response::new(Self::to_proto(row)))
    }

    async fn pay_order(
        &self,
        req: Request<PayOrderRequest>,
    ) -> Result<Response<PayOrderResponse>, Status> {
        let id = &req.into_inner().order_id;
        let row = OrderRepo::get(&self.state.db, id.as_str()).await?;

        let payment_id = row
            .payment_id
            .clone()
            .ok_or_else(|| Status::failed_precondition("no payment for order"))?;
        let payment = self
            .state
            .payment
            .create(
                &row.id.to_string(),
                &row.total_amount.to_string(),
                &row.currency,
                &row.customer_id,
                &format!("order:{}", row.id),
            )
            .await?;

        Ok(Response::new(PayOrderResponse {
            order_id: row.id.to_string(),
            payment_id: payment.id,
            payment_url: payment.payment_url,
            status: payment.status,
        }))
    }
}
