use sqlx::PgPool;
use uuid::Uuid;
use serde_json::Value;

pub async fn publish_batch(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let rows: Vec<(Uuid, String, Value)> = sqlx::query_as(
        r#"SELECT id, event_type, payload FROM outbox_events
           WHERE published_at IS NULL ORDER BY created_at LIMIT 100
           FOR UPDATE SKIP LOCKED"#,
    ).fetch_all(&mut *tx).await?;

    for (id, event_type, payload) in &rows {
        tracing::info!(event_type, payload = %payload, "publishing order event");
        sqlx::query("UPDATE outbox_events SET published_at = now() WHERE id = $1")
            .bind(id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(rows.len() as u64)
}