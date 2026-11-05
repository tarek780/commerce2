// tests/gateway_tests.rs
use hmac::{Hmac, Mac};
use payment_service::gateway::PaymentGateway;
use payment_service::gateway::mock::MockGateway;
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

fn sign(secret: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

pub async fn mock_gateway_create_returns_reference_and_url() {
    let gw = MockGateway {
        public_base: "https://pay.example.com".into(),
        secret: "s".into(),
    };

    let (reference, url) = gw
        .create("order-1", "100.50", "USD")
        .await
        .expect("create failed");

    assert!(
        reference.starts_with("mock_"),
        "unexpected ref: {reference}"
    );
    assert!(url.starts_with("https://pay.example.com/"));
    assert!(url.ends_with(&reference));
}

pub async fn mock_gateway_cancel_is_ok() {
    let gw = MockGateway {
        public_base: "x".into(),
        secret: "s".into(),
    };
    assert!(gw.cancel("mock_abc").await.is_ok());
}

pub fn verify_webhook_accepts_valid_signature() {
    let secret = "super-secret";
    let gw = MockGateway {
        public_base: "x".into(),
        secret: secret.into(),
    };

    let body = br#"{"event_id":"e1","event_type":"payment.succeeded"}"#;
    let sig = sign(secret, body);

    assert!(gw.verify_webhook(body, &sig));
}

pub fn verify_webhook_rejects_tampered_body() {
    let secret = "super-secret";
    let gw = MockGateway {
        public_base: "x".into(),
        secret: secret.into(),
    };

    let body = br#"{"event_id":"e1"}"#;
    let sig = sign(secret, body);

    let tampered = br#"{"event_id":"e2"}"#;
    assert!(!gw.verify_webhook(tampered, &sig));
}

pub fn verify_webhook_rejects_wrong_signature() {
    let gw = MockGateway {
        public_base: "x".into(),
        secret: "right".into(),
    };
    assert!(!gw.verify_webhook(b"body", "deadbeef"));
}

pub fn verify_webhook_rejects_different_length_signature() {
    let gw = MockGateway {
        public_base: "x".into(),
        secret: "s".into(),
    };
    assert!(!gw.verify_webhook(b"body", "short"));
}

pub async fn gateway_master() {
    mock_gateway_create_returns_reference_and_url().await;
    mock_gateway_cancel_is_ok().await;
    verify_webhook_accepts_valid_signature();
    verify_webhook_rejects_different_length_signature();
    verify_webhook_rejects_tampered_body();
    verify_webhook_rejects_wrong_signature();
}
