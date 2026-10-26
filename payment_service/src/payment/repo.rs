use chrono::{DateTime, Utc};
use sqlx::{types::BigDecimal, PgPool};
use uuid::Uuid;

use crate::error::PaymentError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentRow {
    pub id: String,
    pub order_id: String,
    pub idempotency_key: String,
    pub amount: BigDecimal,
    pub currency: String,
    pub status: String,
    pub gateway: String,
    pub gateway_reference: Option<String>,
    pub payment_url: Option<String>,
    pub failure_reason: Option<String>,
    pub customer_id: Option<String>,
    pub metadata: serde_json::Value,
    pub reservation_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PaymentRepo;

impl PaymentRepo {
    pub async fn get(pool: &PgPool, id: String) -> Result<PaymentRow, PaymentError> {
        sqlx::query_as::<_, PaymentRow>("SELECT * FROM payments WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(PaymentError::NotFound)
    }

    pub async fn insert(pool: &PgPool, row: &PaymentRow) -> Result<(), PaymentError> {
        sqlx::query(
            r#"INSERT INTO payments
               (id, order_id, idempotency_key, amount, currency, status, gateway,
                gateway_reference, payment_url, failure_reason, customer_id, metadata,
                reservation_expires_at, created_at, updated_at)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)"#,
        )
        .bind(row.id.clone())
        .bind(row.order_id.clone())
        .bind(&row.idempotency_key)
        .bind(&row.amount)
        .bind(&row.currency)
        .bind(&row.status)
        .bind(&row.gateway)
        .bind(&row.gateway_reference)
        .bind(&row.payment_url)
        .bind(&row.failure_reason)
        .bind(&row.customer_id)
        .bind(&row.metadata)
        .bind(row.reservation_expires_at)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => PaymentError::Duplicate,
            other => PaymentError::Database(other),
        })?;
        Ok(())
    }
    // convert in payment and (update) + store event to outbox_events
    pub async fn transition(
        pool: &PgPool,
        id: String,
        from: &[&str],
        to: &str,
        reference: Option<&str>,
        url: Option<&str>,
        failure: Option<&str>,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<PaymentRow, PaymentError> {
        let mut tx = pool.begin().await?;

        let row = sqlx::query_as::<_, PaymentRow>(
            r#"UPDATE payments SET
                 status = $3,
                 gateway_reference = COALESCE($4, gateway_reference),
                 payment_url       = COALESCE($5, payment_url),
                 failure_reason    = COALESCE($6, failure_reason),
                 updated_at = now()
               WHERE id = $1 AND status = ANY($2)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(from)
        .bind(to)
        .bind(reference)
        .bind(url)
        .bind(failure)
        .fetch_optional(&mut *tx)
        .await?;

        let row = row.ok_or_else(|| PaymentError::InvalidState(format!("cannot {from:?}→{to}")))?;

        sqlx::query(
            "INSERT INTO outbox_events (id, aggregate_type, aggregate_id, event_type, payload)
             VALUES ($1,'payment',$2,$3,$4)",
        )
        .bind(Uuid::now_v7())
        .bind(id)
        .bind(event_type)
        .bind(payload)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    pub async fn find_expired(pool: &PgPool, limit: i64) -> Result<Vec<PaymentRow>, PaymentError> {
        Ok(sqlx::query_as::<_, PaymentRow>(
            r#"SELECT * FROM payments
               WHERE status IN ('pending','processing')
                 AND reservation_expires_at < now()
               ORDER BY reservation_expires_at ASC LIMIT $1
               FOR UPDATE SKIP LOCKED"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?)
    }
}
