use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use rdkafka::ClientConfig;
use serde_json::json;
use std::time::Duration;
use tracing::{error, warn};

#[derive(Clone)]
pub struct KafkaPublisher {
    producer: FutureProducer,
    topic: String,
}

impl KafkaPublisher {
    pub fn new(brokers: &str, topic: &str) -> Result<Self, String> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "zstd")
            .set("acks", "all") // ضمان وصول الحدث
            .set("enable.idempotence", "true")
            .create()
            .map_err(|e| format!("kafka producer create: {e}"))?;

        Ok(Self {
            producer,
            topic: topic.to_string(),
        })
    }

    pub async fn publish_payment_event(
        &self,
        event_type: &str, // payment.created | payment.succeeded | ...
        payment_id: String,
        order_id: Option<String>,
        status: &str,
        reason: Option<&str>,
        data: serde_json::Value,
    ) -> bool {
        let body = json!({
            "event":       event_type,
            "payment_id":  &payment_id,
            "order_id":    order_id,
            "status":      status,
            "reason":      reason,
            "data":        data,
            "occurred_at": chrono::Utc::now().to_rfc3339(),
        });

        let payload = match serde_json::to_vec(&body) {
            Ok(p) => p,
            Err(e) => {
                error!(?e, "serialize payment event");
                return false;
            }
        };

        let record = FutureRecord::<String, Vec<u8>>::to(&self.topic)
            .key(&payment_id)
            .payload(&payload);

        match self
            .producer
            .send(record, Timeout::After(Duration::from_millis(5000)))
            .await
        {
            Ok((partition, offset)) => {
                tracing::debug!(
                    payment_id = %payment_id,
                    event = event_type,
                    partition,
                    offset,
                    "payment event sent to kafka"
                );
                true
            }
            Err((e, _)) => {
                // Note : this may be not error because we have outbox_event to retry later..
                warn!(
                    ?e,
                    payment_id = %payment_id,
                    event = event_type,
                    "instant kafka publish failed; outbox will retry"
                );
                false
            }
        }
    }
}
