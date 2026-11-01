use std::sync::Arc;
use tracing::{info, warn};

use crate::order::orchestrator;
use crate::order::repo::OrderRepo;
use crate::state::AppState;

pub async fn expire_batch(state: &Arc<AppState>) -> Result<u64, sqlx::Error> {
    let rows = match OrderRepo::list_expired(&state.db, 100).await {
        Ok(r) => r,
        Err(e) => {
            warn!(?e, "list_expired failed");
            return Ok(0);
        }
    };

    let mut n = 0u64;
    for row in rows {
        match orchestrator::cancel(state, &row.id.as_str(), "expired_ttl").await {
            Ok(_) => {
                info!(order_id = %row.id, "order expired");
                n += 1;
            }
            Err(e) => warn!(order_id = %row.id, ?e, "expire failed"),
        }
    }
    Ok(n)
}
