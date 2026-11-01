mod common;

use inventory_service::error::InventoryError;

#[tokio::test]
async fn reserve_release_and_available_counts() {
    let env = require_db!();
    let vid = common::new_variant_id();
    common::seed_inventory(&env.pool, vid, 10).await;

    let key = common::new_reservation_key();
    let res = env.repo.reserve(&key, vid, 3, 60).await.unwrap();
    assert_eq!(res.variant_id, vid);
    assert_eq!(res.quantity, 3);

    let (stock, reserved) = env.repo.get_stock(vid).await.unwrap();
    assert_eq!(stock, 10);
    assert_eq!(reserved, 3);

    // release
    env.repo.release(res.id).await.unwrap();
    let (stock, reserved) = env.repo.get_stock(vid).await.unwrap();
    assert_eq!(stock, 10);
    assert_eq!(reserved, 0);

    // release 
    let err = env.repo.release(res.id).await.unwrap_err();
    assert!(matches!(err, InventoryError::InvalidState(_)));

    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn reserve_insufficient_stock() {
    let env = require_db!();
    let vid = common::new_variant_id();
    common::seed_inventory(&env.pool, vid, 2).await;

    let err = env
        .repo
        .reserve(&common::new_reservation_key(), vid, 5, 60)
        .await
        .unwrap_err();
    match err {
        InventoryError::InsufficientStock { available } => assert_eq!(available, 2),
        other => panic!("unexpected error: {other:?}"),
    }

    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn reserve_unknown_variant() {
    let env = require_db!();
    let vid = common::new_variant_id();
    // لا نُهيّئ variant
    let err = env
        .repo
        .reserve(&common::new_reservation_key(), vid, 1, 60)
        .await
        .unwrap_err();
    assert!(matches!(err, InventoryError::VariantNotFound));
}

#[tokio::test]
async fn reserve_idempotent_same_params_returns_same_id() {
    let env = require_db!();
    let vid = common::new_variant_id();
    common::seed_inventory(&env.pool, vid, 5).await;

    let key = common::new_reservation_key();
    let a = env.repo.reserve(&key, vid, 2, 60).await.unwrap();
    let b = env.repo.reserve(&key, vid, 2, 60).await.unwrap();
    assert_eq!(a.id, b.id);

    let (_, reserved) = env.repo.get_stock(vid).await.unwrap();
    assert_eq!(reserved, 2, "reserved يجب أن يُحسب مرة واحدة فقط");

    common::cleanup_variant(&env.pool, vid).await;
}

#[tokio::test]
async fn reserve_idempotency_mismatch_rejects() {
    let env = require_db!();
    let vid = common::new_variant_id();
    common::seed_inventory(&env.pool, vid, 5).await;

    let key = common::new_reservation_key();
    env.repo.reserve(&key, vid, 2, 60).await.unwrap();

    let err = env.repo.reserve(&key, vid, 4, 60).await.unwrap_err();
    assert!(matches!(err, InventoryError::IdempotencyMismatch));

    common::cleanup_variant(&env.pool, vid).await;
}