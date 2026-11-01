use crate::pb::StockUpdate;
use chrono::Utc;
use sqlx::PgPool;
use tokio::sync::broadcast;

pub async fn expire_batch(
    pool: &PgPool,
    tx: &broadcast::Sender<StockUpdate>,
) -> Result<u64, sqlx::Error> {
    let mut db_tx = pool.begin().await?;

    let rows: Vec<(uuid::Uuid, i32, i32)> = sqlx::query_as(
        r#"SELECT id, variant_id, quantity FROM reservations
           WHERE status = 'pending' AND expires_at < now()
           ORDER BY expires_at LIMIT 100 FOR UPDATE SKIP LOCKED"#,
    )
    .fetch_all(&mut *db_tx)
    .await?;

    for (id, variant_id, qty) in &rows {
        sqlx::query("UPDATE inventory SET reserved_quantity = reserved_quantity - $1, updated_at = now() WHERE variant_id = $2")
            .bind(qty).bind(variant_id).execute(&mut *db_tx).await?;
        sqlx::query("UPDATE reservations SET status = 'released' WHERE id = $1")
            .bind(id)
            .execute(&mut *db_tx)
            .await?;
        sqlx::query("INSERT INTO inventory_logs (sku, change_amount, type, reference_id) VALUES ($1, $2, 'expire', $3)")
            .bind(variant_id.to_string()).bind(qty).bind(id.to_string())
            .execute(&mut *db_tx).await?;

        let _ = tx.send(StockUpdate {
            variant_id: *variant_id,
            available_quantity: 0,
            reserved_quantity: 0,
            event_type: "expired".into(),
            updated_at_unix: Utc::now().timestamp(),
        });
    }

    db_tx.commit().await?;
    Ok(rows.len() as u64)
}
