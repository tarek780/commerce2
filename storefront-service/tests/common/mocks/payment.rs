use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use storefront_service::clients::payment::pb_payment::PaymentResponse;
use storefront_service::clients::payment::PaymentClient;
use storefront_service::error::{AppError, AppResult};

#[derive(Default)]
pub struct MockPaymentClient {
    pub payments: Arc<Mutex<Vec<PaymentResponse>>>,
}

impl MockPaymentClient {
    pub fn new() -> Self { Self::default() }

    pub async fn with_payment(mut self, p: PaymentResponse) -> Self {
        self.payments.lock().await.push(p);
        self
    }
}

#[async_trait]
impl PaymentClient for MockPaymentClient {
    async fn get_payment(&self, payment_id: &str) -> AppResult<PaymentResponse> {
        self.payments.lock().await.iter()
            .find(|p| p.id == payment_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("payment not found".into()))
    }
}