use crate::common::*;
use inventory_service::inventory::expiry::expire_batch;
use std::{f32::consts::E, sync::Arc};
use uuid::Uuid;

pub async fn expiry_worker_releases_and_emits_event(env: Arc<TestEnv>) {
    let vid = Uuid::now_v7();

    seed_inventory(&env.pool, vid, 5).await;

    let res = match env.repo.reserve(&new_reservation_key(), vid, 2, -10).await {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };

    let mut rx = env.stock_tx.subscribe();

    let n = expire_batch(&env.pool, &env.stock_tx).await.unwrap();
    assert!(n >= 1, "expected at least one reservation expired");

    let (stock, reserved) = env.repo.get_stock(vid).await.unwrap();
    assert_eq!(stock, 5);
    assert_eq!(reserved, 0);

    let status: (String,) = match sqlx::query_as("SELECT status FROM reservations WHERE id = $1")
        .bind(res.id)
        .fetch_one(&env.pool)
        .await
    {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };
    assert_eq!(status.0, "released");

    let mut got = false;
    for _ in 0..16 {
        match tokio::time::timeout(std::time::Duration::from_millis(200), rx.recv()).await {
            Ok(Ok(ev)) if ev.event_type == "expired" && ev.variant_id == vid.to_string() => {
                got = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(got, "expected 'expired' broadcast event");

    cleanup_variant(&env.pool, vid).await;
}
