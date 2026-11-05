use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use tonic::transport::Channel;

pub mod pb_order {
    tonic::include_proto!("order.v1");
}
use pb_order::order_service_client::OrderServiceClient;
//ListOrdersRequest
use pb_order::{
    CancelOrderRequest, CreateOrderRequest, GetOrderRequest, OrderItem as PbOrderItem,
    OrderResponse, PayOrderRequest, PayOrderResponse,
};
use uuid::Uuid;

#[async_trait]
pub trait OrderClient: Send + Sync + 'static {
    async fn create_order(
        &self,
        customer_id: &str,
        currency: &str,
        items: Vec<(Uuid, i32, String)>,
        idempotency_key: &str,
    ) -> AppResult<OrderResponse>;

    async fn get_order(&self, order_id: &str) -> AppResult<OrderResponse>;
    async fn cancel_order(&self, order_id: &str, reason: &str) -> AppResult<OrderResponse>;
    async fn pay_order(&self, order_id: &str) -> AppResult<PayOrderResponse>;
    // async fn list_orders(
    //     &self, customer_id: &str, first: i32, after: &str, status: &str,
    // ) -> AppResult<pb_order::ListOrdersResponse>;
}

pub struct GrpcOrderClient {
    inner: OrderServiceClient<Channel>,
}

impl GrpcOrderClient {
    pub async fn new(url: &str) -> Result<Self, AppError> {
        let channel = Channel::from_shared(url.to_string())
            .map_err(|e| AppError::Config(format!("order url: {e}")))?
            .connect()
            .await
            .map_err(|e| AppError::Upstream(format!("order connect: {e}")))?;
        Ok(Self {
            inner: OrderServiceClient::new(channel),
        })
    }
}

#[tonic::async_trait]
impl OrderClient for GrpcOrderClient {
    async fn create_order(
        &self,
        customer_id: &str,
        currency: &str,
        items: Vec<(Uuid, i32, String)>,
        idempotency_key: &str,
    ) -> AppResult<OrderResponse> {
        let mut c = self.inner.clone();

        let req = CreateOrderRequest {
            customer_id: customer_id.into(),
            currency: currency.into(),
            items: items
                .into_iter()
                .map(|(vid, qty, price)| PbOrderItem {
                    variant_id: vid.into(),
                    quantity: qty,
                    unit_price: price,
                })
                .collect(),
            idempotency_key: idempotency_key.into(),
        };
        c.create_order(req)
            .await
            .map(|r| r.into_inner())
            .map_err(map_grpc_err)
    }

    async fn get_order(&self, order_id: &str) -> AppResult<OrderResponse> {
        let mut c = self.inner.clone();
        c.get_order(GetOrderRequest {
            order_id: order_id.into(),
        })
        .await
        .map(|r| r.into_inner())
        .map_err(map_grpc_err)
    }

    async fn cancel_order(&self, order_id: &str, reason: &str) -> AppResult<OrderResponse> {
        let mut c = self.inner.clone();
        c.cancel_order(CancelOrderRequest {
            order_id: order_id.into(),
            reason: reason.into(),
        })
        .await
        .map(|r| r.into_inner())
        .map_err(map_grpc_err)
    }

    async fn pay_order(&self, order_id: &str) -> AppResult<PayOrderResponse> {
        let mut c = self.inner.clone();
        c.pay_order(PayOrderRequest {
            order_id: order_id.into(),
        })
        .await
        .map(|r| r.into_inner())
        .map_err(map_grpc_err)
    }

    //     async fn list_orders(
    //         &self,
    //         customer_id: &str,
    //         first: i32,
    //         after: &str,
    //         status: &str,
    //     ) -> AppResult<pb_order::ListOrdersResponse> {
    //         let mut c = self.inner.clone();
    //         c.list_orders(ListOrdersRequest {
    //             customer_id: customer_id.into(),
    //             first,
    //             after: after.into(),
    //             status: status.into(),
    //         })
    //         .await
    //         .map(|r| r.into_inner())
    //         .map_err(map_grpc_err)
    //     }
    // }
}
fn map_grpc_err(s: tonic::Status) -> AppError {
    use tonic::Code;
    match s.code() {
        Code::NotFound => AppError::NotFound(s.message().into()),
        Code::InvalidArgument => AppError::Validation(s.message().into()),
        Code::AlreadyExists => AppError::Conflict(s.message().into()),
        Code::FailedPrecondition => {
            if s.message().contains("insufficient stock") {
                AppError::InsufficientStock {
                    variant_id: 0,
                    requested: 0,
                    available: 0,
                }
            } else {
                AppError::Conflict(s.message().into())
            }
        }
        Code::Unauthenticated => AppError::Unauthorized(s.message().into()),
        Code::Unavailable | Code::DeadlineExceeded => AppError::Upstream(s.message().into()),
        _ => AppError::Upstream(s.message().into()),
    }
}
