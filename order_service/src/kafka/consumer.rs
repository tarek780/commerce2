use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use rdkafka::Message;
use serde::Deserialize;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::order::orchestrator;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PaymentEvent {
    pub event: String, // payment.succeeded | payment.failed | payment.expired ...
    pub payment_id: String,
    pub order_id: String,
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        let consumer: StreamConsumer = match ClientConfig::new()
            .set("bootstrap.servers", &state.config.kafka_brokers)
            .set("group.id", &state.config.kafka_payment_group)
            .set("auto.offset.reset", "earliest")
            .set("enable.auto.commit", "false")
            .create()
        {
            Ok(c) => c,
            Err(e) => {
                error!(?e, "kafka create");
                return;
            }
        };

        if let Err(e) = consumer.subscribe(&[&state.config.kafka_payment_topic]) {
            error!(?e, "subscribe");
            return;
        }
        info!("order-service consuming payment.events");

        loop {
            match consumer.recv().await {
                Ok(m) => {
                    let payload = match m.payload() {
                        Some(p) => p,
                        None => {
                            let _ = consumer.commit_message(&m, CommitMode::Async);
                            continue;
                        }
                    };
                    match serde_json::from_slice::<PaymentEvent>(payload) {
                        Ok(ev) => handle_event(&state, ev).await,
                        Err(e) => warn!(?e, "bad payment event"),
                    }
                    let _ = consumer.commit_message(&m, CommitMode::Async);
                }
                Err(e) => error!(?e, "kafka recv"),
            }
        }
    });
}

async fn handle_event(state: &Arc<AppState>, ev: PaymentEvent) {
    match ev.event.as_str() {
        "payment.succeeded" => {
            match orchestrator::confirm_after_payment(
                state,
                &ev.order_id,
                &ev.payment_id.to_string(),
            )
            .await
            {
                Ok(o) => info!(order_id = %o.id, "order confirmed"),
                Err(e) => warn!(?e, "confirm failed"),
            }
        }
        "payment.failed" | "payment.expired" | "payment.cancelled" => {
            let reason = ev.reason.unwrap_or(ev.event.clone());
            match orchestrator::fail_after_payment(
                state,
                &ev.order_id.as_str(),
                &ev.payment_id.to_string(),
                &reason,
            )
            .await
            {
                Ok(o) => info!(order_id = %o.id, "order failed"),
                Err(e) => warn!(?e, "fail after payment"),
            }
        }
        other => warn!(event = other, "unknown payment event"),
    }
}
