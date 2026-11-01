use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::OrderError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderRow {
    pub id: String,
    pub customer_id: String,
    pub idempotency_key: String,
    pub status: String,
    pub total_amount: sqlx::types::BigDecimal,
    pub currency: String,
    pub reservation_id: Option<String>,
    pub payment_id: Option<String>,
    pub failure_reason: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderItemRow {
    pub id: i64,
    pub order_id: String,
    pub variant_id: i32,
    pub quantity: i32,
    pub unit_price: sqlx::types::BigDecimal,
}

pub struct OrderRepo;

impl OrderRepo {
    pub async fn get(pool: &PgPool, id: &str) -> Result<OrderRow, OrderError> {
        sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(OrderError::NotFound)
    }

    pub async fn items(pool: &PgPool, order_id: &str) -> Result<Vec<OrderItemRow>, OrderError> {
        Ok(sqlx::query_as::<_, OrderItemRow>(
            "SELECT * FROM order_items WHERE order_id = $1 ORDER BY id",
        )
        .bind(order_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn insert(
        pool: &PgPool,
        row: &OrderRow,
        items: &[(i32, i32, sqlx::types::BigDecimal)],
    ) -> Result<(), OrderError> {
        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO orders
               (id, customer_id, idempotency_key, status, total_amount, currency,
                expires_at, created_at, updated_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
        )
        .bind(&row.id)
        .bind(&row.customer_id)
        .bind(&row.idempotency_key)
        .bind(&row.status)
        .bind(&row.total_amount)
        .bind(&row.currency)
        .bind(row.expires_at)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => OrderError::Duplicate,
            other => OrderError::Database(other),
        })?;

        for (variant_id, qty, price) in items {
            sqlx::query(
                "INSERT INTO order_items (order_id, variant_id, quantity, unit_price)
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(&row.id)
            .bind(variant_id)
            .bind(qty)
            .bind(price)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn transition(
        pool: &PgPool,
        id: &str,
        from: &[&str],
        to: &str,
        set_reservation: Option<&str>,
        set_payment: Option<&str>,
        failure_reason: Option<&str>,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<OrderRow, OrderError> {
        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, OrderRow>(
            r#"UPDATE orders SET
                 status = $3,
                 reservation_id = COALESCE($4, reservation_id),
                 payment_id     = COALESCE($5, payment_id),
                 failure_reason = COALESCE($6, failure_reason),
                 updated_at = now()
               WHERE id = $1 AND status = ANY($2)
               RETURNING *"#,
        )
        .bind(id)
        .bind(from)
        .bind(to)
        .bind(set_reservation)
        .bind(set_payment)
        .bind(failure_reason)
        .fetch_optional(&mut *tx)
        .await?;

        let row = row.ok_or_else(|| OrderError::InvalidState(format!("cannot {from:?}→{to}")))?;

        sqlx::query(
            "INSERT INTO outbox_events (id, aggregate_type, aggregate_id, event_type, payload)
             VALUES ($1,'order',$2,$3,$4)",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(id)
        .bind(event_type)
        .bind(payload)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    pub async fn list_expired(pool: &PgPool, limit: i64) -> Result<Vec<OrderRow>, OrderError> {
        Ok(sqlx::query_as::<_, OrderRow>(
            r#"SELECT * FROM orders
               WHERE status IN ('pending','reserved','awaiting_payment')
                 AND expires_at < now()
               ORDER BY expires_at ASC LIMIT $1
               FOR UPDATE SKIP LOCKED"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?)
    }
}
