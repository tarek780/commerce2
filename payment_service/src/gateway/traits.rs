use async_trait::async_trait;

#[async_trait]
pub trait PaymentGateway: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    async fn create(
        &self,
        order_id: &str,
        amount: &str,
        currency: &str,
    ) -> Result<(String /*reference*/, String /*url*/), String>;

    async fn cancel(&self, reference: &str) -> Result<(), String>;

    fn verify_webhook(&self, body: &[u8], signature: &str) -> bool;
}
