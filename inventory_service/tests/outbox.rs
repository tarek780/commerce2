mod common;

use inventory_service::outbox::publisher::publish_batch;

#[tokio::test]
async fn outbox_marks_events_as_published() {
    let env = require_db!();
    let vid = common::new_variant_id();
    common::seed_inventory(&env.pool, vid, 10).await;


    let _ = env
        .repo
        .reserve(&common::new_reservation_key(), vid, 1, 60)
        .await
        .unwrap();


    let pending: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM outbox_events WHERE published_at IS NULL AND aggregate_id = $1",
    )
    .bind(vid as i64)
    .fetch_one(&env.pool)
    .await
    .unwrap();
    assert!(pending.0 >= 1);


    let n = publish_batch(&env.pool).await.unwrap();
    assert!(n >= 1);


    let pending_after: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM outbox_events WHERE published_at IS NULL AND aggregate_id = $1",
    )
    .bind(vid as i64)
    .fetch_one(&env.pool)
    .await
    .unwrap();
    assert_eq!(pending_after.0, 0);


    let _ = sqlx::query("DELETE FROM outbox_events WHERE aggregate_id = $1")
        .bind(vid as i64)
        .execute(&env.pool)
        .await;
    common::cleanup_variant(&env.pool, vid).await;
}