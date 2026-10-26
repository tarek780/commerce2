use chrono::{Duration, Utc};
use sqlx::types::BigDecimal;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::OrderError;
use crate::order::repo::{OrderRepo, OrderRow};
use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct NewItem {
    pub variant_id: i32,
    pub quantity: i32,
    pub unit_price: BigDecimal,
}

pub async fn create_and_orchestrate(
    state: &AppState,
    customer_id: &str,
    currency: &str,
    items: Vec<NewItem>,
    idempotency_key: &str,
) -> Result<OrderRow, OrderError> {
    if items.is_empty() {
        return Err(OrderError::InvalidState("empty items".into()));
    }

    let mut total = BigDecimal::from(0);
    for it in &items {
        total += &it.unit_price * BigDecimal::from(it.quantity);
    }

    let now = Utc::now();
    let row = OrderRow {
        id: Uuid::now_v7().to_string(),
        customer_id: customer_id.into(),
        idempotency_key: idempotency_key.into(),
        status: "pending".into(),
        total_amount: total.clone(),
        currency: currency.into(),
        reservation_id: None,
        payment_id: None,
        failure_reason: None,
        expires_at: now + Duration::seconds(state.config.order_ttl_secs),
        created_at: now,
        updated_at: now,
    };

    let items_for_db: Vec<(i32, i32, BigDecimal)> = items
        .iter()
        .map(|i| (i.variant_id, i.quantity, i.unit_price.clone()))
        .collect();

    OrderRepo::insert(&state.db, &row, &items_for_db).await?;
    // reservation
    let mut reservation_id = String::new();
    let ttl = state.config.order_ttl_secs;
    for it in &items {
        let key = format!("order:{}:{}", row.id, it.variant_id);
        match state
            .inventory
            .reserve(&key, it.variant_id, it.quantity, ttl)
            .await
        {
            Ok((rid, _)) => {
                reservation_id = rid;
            }
            Err(status) => {
                warn!(variant_id = it.variant_id, %status, "inventory reserve failed");
                let _ = OrderRepo::transition(
                    &state.db,
                    &row.id.as_str(),
                    &["pending"],
                    "failed",
                    None,
                    None,
                    Some(&status.message()),
                    "order.failed",
                    serde_json::json!({"reason": status.message()}),
                )
                .await;
                return Err(OrderError::Inventory(status.message().to_string()));
            }
        }
    }

    // 3) reserved
    let row = OrderRepo::transition(
        &state.db,
        &row.id.as_str(),
        &["pending"],
        "reserved",
        Some(&reservation_id),
        None,
        None,
        "order.reserved",
        serde_json::json!({"reservation_id": reservation_id}),
    )
    .await?;

    // 4) اطلب دفع
    match state
        .payment
        .create(
            &row.id.to_string(),
            &row.total_amount.to_string(),
            &row.currency,
            &row.customer_id,
            &format!("order:{}", row.id),
        )
        .await
    {
        Ok(p) => {
            let row = OrderRepo::transition(
                &state.db,
                &row.id.as_str(),
                &["reserved"],
                "awaiting_payment",
                None,
                Some(&p.id),
                None,
                "order.awaiting_payment",
                serde_json::json!({
                    "payment_id": p.id,
                    "payment_url": p.payment_url,
                }),
            )
            .await?;
            info!(order_id = %row.id, payment_id = %p.id, "order awaiting payment");
            Ok(row)
        }
        Err(status) => {
            warn!(%status, "payment create failed; releasing inventory");
            let _ = state.inventory.release(&reservation_id).await;
            let _ = OrderRepo::transition(
                &state.db,
                &row.id.as_str(),
                &["reserved"],
                "failed",
                None,
                None,
                Some(&status.message()),
                "order.failed",
                serde_json::json!({"reason": status.message()}),
            )
            .await;
            Err(OrderError::Payment(status.message().to_string()))
        }
    }
}

/// إلغاء الطلب (من المستخدم أو من expiry worker)
pub async fn cancel(
    state: &AppState,
    order_id: &str,
    reason: &str,
) -> Result<OrderRow, OrderError> {
    let row = OrderRepo::get(&state.db, order_id).await?;

    if matches!(
        row.status.as_str(),
        "paid" | "confirmed" | "cancelled" | "failed" | "expired"
    ) {
        return Err(OrderError::InvalidState(row.status));
    }

    // ألغِ الدفع إن وُجد
    if let Some(pid) = &row.payment_id {
        let _ = state.payment.cancel(pid, reason).await;
    }
    // حرّر المخزون
    if let Some(rid) = &row.reservation_id {
        let _ = state.inventory.release(rid).await;
    }

    OrderRepo::transition(
        &state.db,
        &row.id.as_str(),
        &["pending", "reserved", "awaiting_payment"],
        "cancelled",
        None,
        None,
        Some(reason),
        "order.cancelled",
        serde_json::json!({"reason": reason}),
    )
    .await
}

/// عند نجاح الدفع من Kafka: commit inventory + confirmed
pub async fn confirm_after_payment(
    state: &AppState,
    order_id: &str,
    payment_id: &str,
) -> Result<OrderRow, OrderError> {
    let row = OrderRepo::get(&state.db, order_id).await?;
    if row.status != "awaiting_payment" {
        return Err(OrderError::InvalidState(row.status));
    }
    if row.payment_id.as_deref() != Some(payment_id) {
        return Err(OrderError::InvalidState("payment_id mismatch".into()));
    }

    // التزام المخزون نهائياً
    if let Some(rid) = &row.reservation_id {
        state
            .inventory
            .commit(rid)
            .await
            .map_err(|s| OrderError::Inventory(s.message().into()))?;
    }

    OrderRepo::transition(
        &state.db,
        &row.id.as_str(),
        &["awaiting_payment"],
        "confirmed",
        None,
        None,
        None,
        "order.confirmed",
        serde_json::json!({"payment_id": payment_id}),
    )
    .await
}

/// عند فشل الدفع: حرّر المخزون + failed
pub async fn fail_after_payment(
    state: &AppState,
    order_id: &str,
    payment_id: &str,
    reason: &str,
) -> Result<OrderRow, OrderError> {
    let row = OrderRepo::get(&state.db, order_id).await?;
    if row.status != "awaiting_payment" {
        return Err(OrderError::InvalidState(row.status));
    }

    if let Some(rid) = &row.reservation_id {
        let _ = state.inventory.release(rid).await;
    }

    OrderRepo::transition(
        &state.db,
        &row.id.as_str(),
        &["awaiting_payment"],
        "failed",
        None,
        None,
        Some(reason),
        "order.failed",
        serde_json::json!({"payment_id": payment_id, "reason": reason}),
    )
    .await
}
