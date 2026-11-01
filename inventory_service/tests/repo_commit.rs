mod common;
use inventory_service::error::InventoryError;
use uuid::Uuid;

#[tokio::test]
async fn commit_decrements_stock_and_reserved() {
    let env = require_db!();
    let vid = Uuid::now_v7();
    common::seed_inventory(&env.pool, vid, 10).await;

    let res = env
        .repo
        .reserve(&common::new_reservation_key(), vid, 4, 60)
        .await
        .unwrap();
    env.repo.commit(res.id).await.unwrap();

    let (stock, reserved) = env.repo.get_stock(vid).await.unwrap();
    assert_eq!(stock, 6);
    assert_eq!(reserved, 0);

    // commit
    let err = env.repo.commit(res.id).await.unwrap_err();
    assert!(matches!(err, InventoryError::InvalidState(_)));

    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn commit_expired_reservation_fails() {
    let env = require_db!();
    let vid = Uuid::now_v7();
    common::seed_inventory(&env.pool, vid, 10).await;

    // TTL
    let res = env
        .repo
        .reserve(&common::new_reservation_key(), vid, 1, -5)
        .await
        .unwrap();

    let err = env.repo.commit(res.id).await.unwrap_err();
    assert!(matches!(err, InventoryError::ReservationExpired));

    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn adjust_stock_success_and_negative_guard() {
    let env = require_db!();
    let vid = Uuid::now_v7();
    common::seed_inventory(&env.pool, vid, 5).await;

    let new_stock = env.repo.adjust(vid, 3, "ref-1").await.unwrap();
    assert_eq!(new_stock, 8);

    let new_stock = env.repo.adjust(vid, -2, "ref-2").await.unwrap();
    assert_eq!(new_stock, 6);

    let err = env.repo.adjust(vid, -100, "ref-3").await.unwrap_err();
    assert!(matches!(err, InventoryError::InvalidState(_)));

    common::cleanup_variant(&env.pool, vid).await;
}
