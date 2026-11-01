use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use rdkafka::ClientConfig;
use serde_json::Value;
use sqlx::PgPool;
use std::time::Duration;

pub async fn publish_batch(pool: &PgPool, topic: &str) -> Result<u64, sqlx::Error> {
    //
    let producer: FutureProducer = ClientConfig::new()
        .set(
            "bootstrap.servers",
            std::env::var("KAFKA_BROKERS").unwrap_or("localhost:9092".into()),
        )
        .set("message.timeout.ms", "5000")
        .create()
        .map_err(|e| sqlx::Error::Protocol(format!("kafka: {e}")))?;
    //
    let mut tx = pool.begin().await?;
    //
    let rows: Vec<(String, String, String, Value)> = sqlx::query_as(
        r#"SELECT id, event_type, aggregate_id, payload FROM outbox_events
           WHERE published_at IS NULL ORDER BY created_at LIMIT 100
           FOR UPDATE SKIP LOCKED"#,
    )
    .fetch_all(&mut *tx)
    .await?;

    let mut n = 0u64;

    for (id, event_type, aggregate_id, payload) in &rows {
        let body = serde_json::json!({
            "event": event_type,
            "payment_id": aggregate_id,
            "order_id": payload.get("order_id"),
            "status": payload.get("status"),
            "reason": payload.get("reason"),
            "data": payload,

        });
        let bytes = serde_json::to_vec(&body).unwrap();

        let record: FutureRecord<'_, String, Vec<u8>> = FutureRecord::<String, Vec<u8>>::to(topic)
            .key(&aggregate_id)
            .payload(&bytes);

        match producer
            .send(record, Timeout::After(Duration::from_millis(5000)))
            .await
        {
            Ok(_) => {
                sqlx::query("UPDATE outbox_events SET published_at = now() WHERE id = $1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                n += 1;
            }
            Err((e, _)) => {
                tracing::error!(?e, "kafka publish failed");
                break;
            }
        }
    }

    tx.commit().await?;
    Ok(n)
}
