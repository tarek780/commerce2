use crate::common::*;
use inventory_service::outbox::publisher::publish_batch;
use std::sync::Arc;
use uuid::Uuid;

pub async fn outbox_marks_events_as_published(env: Arc<TestEnv>) {
    let vid = Uuid::now_v7();
    seed_inventory(&env.pool, vid, 10).await;

    let _ = env
        .repo
        .reserve(&new_reservation_key(), vid, 1, 60)
        .await
        .unwrap();

    let pending: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM outbox_events WHERE published_at IS NULL AND aggregate_id = $1",
    )
    .bind(vid.to_string())
    .fetch_one(&env.pool)
    .await
    .unwrap();
    assert!(pending.0 >= 1);

    let n = publish_batch(&env.pool).await.unwrap();
    assert!(n >= 1);

    let pending_after: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM outbox_events WHERE published_at IS NULL AND aggregate_id = $1",
    )
    .bind(vid.to_string())
    .fetch_one(&env.pool)
    .await
    .unwrap();
    assert_eq!(pending_after.0, 0);

    let _ = sqlx::query("DELETE FROM outbox_events WHERE aggregate_id = $1")
        .bind(vid.to_string())
        .execute(&env.pool)
        .await;
    cleanup_variant(&env.pool, vid).await;
}
