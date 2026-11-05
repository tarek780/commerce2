use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use storefront_service::clients::order::OrderClient;
use storefront_service::clients::order::pb_order::{OrderResponse, PayOrderResponse};
use storefront_service::error::{AppError, AppResult};

#[derive(Default)]
pub struct MockOrderClient {
    pub orders: Arc<Mutex<Vec<OrderResponse>>>,
    pub create_response: Arc<Mutex<Option<Result<OrderResponse, AppError>>>>,
    // pub list_response: Arc<Mutex<Option<ListOrdersResponse>>>,
    pub pay_response: Arc<Mutex<Option<PayOrderResponse>>>,
    pub cancel_response: Arc<Mutex<Option<Result<OrderResponse, AppError>>>>,
    pub create_calls: Arc<Mutex<usize>>,
}

impl MockOrderClient {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl OrderClient for MockOrderClient {
    async fn create_order(
        &self,
        _customer_id: &str,
        _currency: &str,
        _items: Vec<(Uuid, i32, String)>,
        _idempotency_key: &str,
    ) -> AppResult<OrderResponse> {
        *self.create_calls.lock().await += 1;
        if let Some(resp) = self.create_response.lock().await.take() {
            return resp;
        }
        Ok(OrderResponse {
            id: "order-1".into(),
            customer_id: "cust".into(),
            status: "awaiting_payment".into(),
            total_amount: "60.00".into(),
            currency: "USD".into(),
            reservation_id: "res-1".into(),
            payment_id: "pay-1".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            failure_reason: None,
        })
    }

    async fn get_order(&self, order_id: &str) -> AppResult<OrderResponse> {
        self.orders
            .lock()
            .await
            .iter()
            .find(|o| o.id == order_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("order not found".into()))
    }

    async fn cancel_order(&self, order_id: &str, _reason: &str) -> AppResult<OrderResponse> {
        if let Some(resp) = self.cancel_response.lock().await.take() {
            return resp;
        }
        let mut orders = self.orders.lock().await;
        if let Some(o) = orders.iter_mut().find(|o| o.id == order_id) {
            o.status = "cancelled".into();
            return Ok(o.clone());
        }
        Err(AppError::NotFound("order not found".into()))
    }

    async fn pay_order(&self, order_id: &str) -> AppResult<PayOrderResponse> {
        if let Some(r) = self.pay_response.lock().await.take() {
            return Ok(r);
        }
        Ok(PayOrderResponse {
            order_id: order_id.into(),
            payment_id: "pay-1".into(),
            payment_url: "http://pay/mock".into(),
            status: "processing".into(),
        })
    }

    // async fn list_orders(
    //     &self,
    //     _customer_id: &str,
    //     _first: i32,
    //     _after: &str,
    //     _status: &str,
    // ) -> AppResult<ListOrdersResponse> {
    //     if let Some(r) = self.list_response.lock().await.take() {
    //         return Ok(r);
    //     }
    //     Ok(ListOrdersResponse {
    //         edges: vec![],
    //         has_next_page: false,
    //         end_cursor: String::new(),
    //     })
    // }
}
