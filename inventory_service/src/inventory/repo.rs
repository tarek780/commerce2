use crate::error::InventoryError;
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Reservation {
    pub id: Uuid,
    pub variant_id: i32,
    pub quantity: i32,
    pub expires_at: chrono::DateTime<Utc>,
}

pub struct InventoryRepo {
    pub pool: PgPool,
}

impl InventoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn reserve(
        &self,
        reservation_key: &str,
        variant_id: i32,
        quantity: i32,
        ttl_seconds: i64,
    ) -> Result<Reservation, InventoryError> {
        let mut tx = self.pool.begin().await?;

        let new_id = Uuid::now_v7();
        let expires_at = Utc::now() + Duration::seconds(ttl_seconds);

        let inserted: Option<(Uuid,)> = sqlx::query_as(
            r#"INSERT INTO reservations (id, reservation_key, variant_id, quantity, status, expires_at)
               VALUES ($1, $2, $3, $4, 'pending', $5)
               ON CONFLICT (reservation_key) DO NOTHING
               RETURNING id"#,
        )
        .bind(new_id).bind(reservation_key).bind(variant_id)
        .bind(quantity).bind(expires_at)
        .fetch_optional(&mut *tx).await?;

        let reservation_id = match inserted {
            Some((id,)) => {
                let row: Option<(i32, i32)> = sqlx::query_as(
                    "SELECT stock_quantity, reserved_quantity FROM inventory WHERE variant_id = $1 FOR UPDATE",
                ).bind(variant_id).fetch_optional(&mut *tx).await?;

                let (stock, reserved) = row.ok_or(InventoryError::VariantNotFound)?;
                let available = stock - reserved;
                if available < quantity {
                    return Err(InventoryError::InsufficientStock { available });
                }

                sqlx::query("UPDATE inventory SET reserved_quantity = reserved_quantity + $1, updated_at = now() WHERE variant_id = $2")
                    .bind(quantity).bind(variant_id).execute(&mut *tx).await?;

                sqlx::query("INSERT INTO inventory_logs (sku, change_amount, type, reference_id) VALUES ($1, $2, 'reserve', $3)")
                    .bind(variant_id.to_string()).bind(-quantity).bind(reservation_key)
                    .execute(&mut *tx).await?;

                sqlx::query("INSERT INTO outbox_events (id, aggregate_type, aggregate_id, event_type, payload) VALUES ($1, 'inventory', $2, 'reserved', $3)")
                    .bind(Uuid::now_v7()).bind(variant_id as i64)
                    .bind(serde_json::json!({"reservation_id": new_id, "variant_id": variant_id, "quantity": quantity}))
                    .execute(&mut *tx).await?;

                id
            }
            None => {
                let existing: Option<(Uuid, i32, i32)> = sqlx::query_as(
                    "SELECT id, variant_id, quantity FROM reservations WHERE reservation_key = $1 AND status = 'pending'",
                ).bind(reservation_key).fetch_optional(&mut *tx).await?;
                let (id, vid, qty) = existing.ok_or(InventoryError::ReservationNotFound)?;
                if vid != variant_id || qty != quantity {
                    return Err(InventoryError::IdempotencyMismatch);
                }
                id
            }
        };

        tx.commit().await?;
        Ok(Reservation {
            id: reservation_id,
            variant_id,
            quantity,
            expires_at,
        })
    }

    pub async fn release(&self, reservation_id: Uuid) -> Result<(), InventoryError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(i32, i32, String)> = sqlx::query_as(
            "SELECT variant_id, quantity, status FROM reservations WHERE id = $1 FOR UPDATE",
        )
        .bind(reservation_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (variant_id, quantity, status) = row.ok_or(InventoryError::ReservationNotFound)?;
        if status != "pending" {
            return Err(InventoryError::InvalidState(status));
        }

        sqlx::query("UPDATE inventory SET reserved_quantity = reserved_quantity - $1, updated_at = now() WHERE variant_id = $2")
            .bind(quantity).bind(variant_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE reservations SET status = 'released' WHERE id = $1")
            .bind(reservation_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO inventory_logs (sku, change_amount, type, reference_id) VALUES ($1, $2, 'release', $3)")
            .bind(variant_id.to_string()).bind(quantity).bind(reservation_id.to_string())
            .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn commit(&self, reservation_id: Uuid) -> Result<(), InventoryError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(i32, i32, String, chrono::DateTime<Utc>)> = sqlx::query_as(
            "SELECT variant_id, quantity, status, expires_at FROM reservations WHERE id = $1 FOR UPDATE",
        ).bind(reservation_id).fetch_optional(&mut *tx).await?;
        let (variant_id, quantity, status, expires_at) =
            row.ok_or(InventoryError::ReservationNotFound)?;
        if status != "pending" {
            return Err(InventoryError::InvalidState(status));
        }
        if expires_at < Utc::now() {
            return Err(InventoryError::ReservationExpired);
        }

        sqlx::query("UPDATE inventory SET stock_quantity = stock_quantity - $1, reserved_quantity = reserved_quantity - $1, updated_at = now() WHERE variant_id = $2")
            .bind(quantity).bind(variant_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE reservations SET status = 'committed' WHERE id = $1")
            .bind(reservation_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO inventory_logs (sku, change_amount, type, reference_id) VALUES ($1, $2, 'commit', $3)")
            .bind(variant_id.to_string()).bind(-quantity).bind(reservation_id.to_string())
            .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_stock(&self, variant_id: i32) -> Result<(i32, i32), InventoryError> {
        let row: Option<(i32, i32)> = sqlx::query_as(
            "SELECT stock_quantity, reserved_quantity FROM inventory WHERE variant_id = $1",
        )
        .bind(variant_id)
        .fetch_optional(&self.pool)
        .await?;
        row.ok_or(InventoryError::VariantNotFound)
    }

    pub async fn adjust(
        &self,
        variant_id: i32,
        delta: i32,
        reference_id: &str,
    ) -> Result<i32, InventoryError> {
        let mut tx = self.pool.begin().await?;
        let row: Option<(i32,)> =
            sqlx::query_as("SELECT stock_quantity FROM inventory WHERE variant_id = $1 FOR UPDATE")
                .bind(variant_id)
                .fetch_optional(&mut *tx)
                .await?;
        let (current,) = row.ok_or(InventoryError::VariantNotFound)?;
        let new_stock = current + delta;
        if new_stock < 0 {
            return Err(InventoryError::InvalidState(
                "stock would go negative".into(),
            ));
        }

        sqlx::query(
            "UPDATE inventory SET stock_quantity = $1, updated_at = now() WHERE variant_id = $2",
        )
        .bind(new_stock)
        .bind(variant_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO inventory_logs (sku, change_amount, type, reference_id) VALUES ($1, $2, 'adjust', $3)")
            .bind(variant_id.to_string()).bind(delta).bind(reference_id)
            .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(new_stock)
    }
}
