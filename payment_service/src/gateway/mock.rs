use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::traits::PaymentGateway;

type HmacSha256 = Hmac<Sha256>;

pub struct MockGateway {
    pub public_base: String,
    pub secret: String,
}

#[async_trait]
impl PaymentGateway for MockGateway {
    fn name(&self) -> &'static str { "mock" }

    async fn create(&self, order_id: &str, amount: &str, _currency: &str)
        -> Result<(String, String), String>
    {
        let mut h = DefaultHasher::new();
        (order_id, amount, chrono::Utc::now().timestamp()).hash(&mut h);
        let reference = format!("mock_{:x}", h.finish());
        let url = format!("{}/{}", self.public_base, reference);
        Ok((reference, url))
    }

    async fn cancel(&self, _reference: &str) -> Result<(), String> { Ok(()) }

    fn verify_webhook(&self, body: &[u8], signature: &str) -> bool {
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        constant_time_eq(expected.as_bytes(), signature.as_bytes())
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut r = 0u8;
    for (x, y) in a.iter().zip(b) { r |= x ^ y; }
    r == 0
}