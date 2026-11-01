use std::collections::HashMap;
use tonic::transport::Channel;
use crate::pb_pay::payment_service_client::PaymentServiceClient;
use crate::pb_pay::{CancelPaymentRequest, CreatePaymentRequest, PaymentResponse};

#[derive(Clone)]
pub struct PaymentClient {
    inner: PaymentServiceClient<Channel>,
}

impl PaymentClient {
    pub fn new(channel: Channel) -> Self {
        Self { inner: PaymentServiceClient::new(channel) }
    }

    pub async fn create(
        &self,
        order_id: &str,
        amount: &str,
        currency: &str,
        customer_id: &str,
        idempotency_key: &str,
    ) -> Result<PaymentResponse, tonic::Status> {
        let mut c = self.inner.clone();
        let mut md = HashMap::new();
        md.insert("source".to_string(), "order-service".to_string());

        c.create_payment(CreatePaymentRequest {
            order_id: order_id.to_string(),
            amount: amount.to_string(),
            currency: currency.to_string(),
            idempotency_key: idempotency_key.to_string(),
            customer_id: customer_id.to_string(),
            metadata: md,
        }).await.map(|r| r.into_inner())
    }

    pub async fn cancel(&self, payment_id: &str, reason: &str) -> Result<PaymentResponse, tonic::Status> {
        let mut c = self.inner.clone();
        c.cancel_payment(CancelPaymentRequest {
            payment_id: payment_id.to_string(),
            reason: reason.to_string(),
        }).await.map(|r| r.into_inner())
    }
}