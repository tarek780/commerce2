pub mod mock;
pub mod traits;

use std::sync::Arc;
use crate::config::PaymentConfig;
pub use traits::PaymentGateway;

pub fn build(cfg: &PaymentConfig) -> Arc<dyn PaymentGateway> {
    match cfg.gateway.as_str() {
        "mock" => Arc::new(mock::MockGateway {
            public_base: cfg.gateway_public_base.clone(),
            secret: cfg.webhook_secret.clone(),
        }),
        _ => Arc::new(mock::MockGateway {
            public_base: cfg.gateway_public_base.clone(),
            secret: cfg.webhook_secret.clone(),
        }),
    }
}