
use sqlx::PgPool;
use std::sync::Arc;

use crate::config::PaymentConfig;
use crate::gateway::PaymentGateway;
use crate::payment::kafka_publisher::KafkaPublisher;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub gateway: Arc<dyn PaymentGateway>,
    pub kafka: Option<Arc<KafkaPublisher>>, 
    pub config: Arc<PaymentConfig>,
}

impl AppState {
    pub async fn new(cfg: PaymentConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .connect(&cfg.database_url)
            .await?;

        let gateway = crate::gateway::build(&cfg);

        let kafka = match KafkaPublisher::new(&cfg.kafka_brokers, &cfg.kafka_payment_topic) {
            Ok(k) => {
                tracing::info!("kafka publisher ready");
                Some(Arc::new(k))
            }
            Err(e) => {
                tracing::warn!(error = %e, "kafka publisher unavailable; using outbox only");
                None
            }
        };

        Ok(Self {
            db,
            gateway,
            kafka,
            config: Arc::new(cfg),
        })
    }

    pub async fn try_publish(
        &self,
        event_type: &str,
        payment_id: String,
        order_id: Option<String>,
        status: &str,
        reason: Option<&str>,
        data: serde_json::Value,
    ) {
        if let Some(k) = &self.kafka {
            let _ = k
                .publish_payment_event(event_type, payment_id, order_id, status, reason, data)
                .await;
        }
    }
}
