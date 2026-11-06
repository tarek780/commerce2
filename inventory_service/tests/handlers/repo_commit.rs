use crate::common::*;
use inventory_service::error::InventoryError;
use std::sync::Arc;
use uuid::Uuid;

pub async fn commit_decrements_stock_and_reserved(env: Arc<TestEnv>) {
    let vid = Uuid::now_v7();
    seed_inventory(&env.pool, vid, 10).await;

    let res = match env.repo.reserve(&new_reservation_key(), vid, 4, 60).await {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };
    match env.repo.commit(res.id).await {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };
    let (stock, reserved) = env.repo.get_stock(vid).await.unwrap();
    assert_eq!(stock, 6);
    assert_eq!(reserved, 0);

    // commit
    let err = env.repo.commit(res.id).await.unwrap_err();
    assert!(matches!(err, InventoryError::InvalidState(_)));

    cleanup_variant(&env.pool, vid).await;
}

pub async fn commit_expired_reservation_fails(env: Arc<TestEnv>) {
    let vid = Uuid::now_v7();
    seed_inventory(&env.pool, vid, 10).await;

    // TTL
    let res = match env.repo.reserve(&new_reservation_key(), vid, 1, -5).await {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };

    let err = env.repo.commit(res.id).await.unwrap_err();
    assert!(matches!(err, InventoryError::ReservationExpired));

    cleanup_variant(&env.pool, vid).await;
}

pub async fn adjust_stock_success_and_negative_guard(env: Arc<TestEnv>) {
    let vid = Uuid::now_v7();
    seed_inventory(&env.pool, vid, 5).await;

    let new_stock = match env.repo.adjust(vid, 3, "ref-1").await {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };
    assert_eq!(new_stock, 8);

    let new_stock = match env.repo.adjust(vid, -2, "ref-2").await {
        Ok(x) => x,
        Err(e) => {
            panic!(" err : {e}");
        }
    };
    assert_eq!(new_stock, 6);

    let err = env.repo.adjust(vid, -100, "ref-3").await.unwrap_err();
    assert!(matches!(err, InventoryError::InvalidState(_)));

    cleanup_variant(&env.pool, vid).await;
}
