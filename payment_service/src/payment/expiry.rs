use std::sync::Arc;
use tracing::{info, warn};

use crate::payment::repo::PaymentRepo;
use crate::state::AppState;

pub async fn expire_batch(state: &Arc<AppState>) -> Result<u64, sqlx::Error> {
    let rows = match PaymentRepo::find_expired(&state.db, 100).await {
        Ok(r) => r,
        Err(e) => {
            warn!(?e, "find_expired failed");
            return Ok(0);
        }
    };
    //

    let mut n = 0u64;
    //
    for row in rows {
        if let Some(reference) = &row.gateway_reference {
            let _ = state.gateway.cancel(reference).await;
        }
        match PaymentRepo::transition(
            &state.db,
            row.id.clone(),
            &["pending", "processing"],
            "expired",
            None,
            None,
            Some("reservation_ttl_exceeded"),
            "payment.expired",
            serde_json::json!({"payment_id": row.id.clone()}),
        )
        .await
        {
            Ok(_) => {
                info!(payment_id = %row.id, "payment expired");
                n += 1;
            }
            Err(e) => warn!(payment_id = %row.id, ?e, "expire failed"),
        }
    }
    Ok(n)
}
