use async_trait::async_trait;
use tonic::transport::Channel;
use crate::error::{AppError, AppResult};

pub mod pb_payment {
    tonic::include_proto!("payment.v1");
}
use pb_payment::payment_service_client::PaymentServiceClient;
use pb_payment::{GetPaymentRequest, PaymentResponse};

#[async_trait]
pub trait PaymentClient: Send + Sync + 'static {
    async fn get_payment(&self, payment_id: &str) -> AppResult<PaymentResponse>;
}

pub struct GrpcPaymentClient {
    inner: PaymentServiceClient<Channel>,
}

impl GrpcPaymentClient {
    pub async fn new(url: &str) -> Result<Self, AppError> {
        let channel = Channel::from_shared(url.to_string())
            .map_err(|e| AppError::Config(format!("payment url: {e}")))?
            .connect()
            .await
            .map_err(|e| AppError::Upstream(format!("payment connect: {e}")))?;
        Ok(Self { inner: PaymentServiceClient::new(channel) })
    }
}

#[async_trait]
impl PaymentClient for GrpcPaymentClient {
    async fn get_payment(&self, payment_id: &str) -> AppResult<PaymentResponse> {
        let mut c = self.inner.clone();
        c.get_payment(GetPaymentRequest { payment_id: payment_id.into() })
            .await
            .map(|r| r.into_inner())
            .map_err(|s| match s.code() {
                tonic::Code::NotFound => AppError::NotFound(s.message().into()),
                _ => AppError::Upstream(s.message().into()),
            })
    }
}