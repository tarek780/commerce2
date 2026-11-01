mod common;

use cart_service::cart::model::AddItemRequest;
use cart_service::error::CartError;
use uuid::Uuid;

#[tokio::test]
async fn create_get_and_add_items_to_guest_cart() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let guest_id = env.state.cart_service.create_guest_cart().await.unwrap();
    assert!(!guest_id.is_empty());

    let cart = env
        .state
        .cart_service
        .get_guest_cart(&guest_id)
        .await
        .unwrap();
    assert!(cart.items.is_empty());

    env.state
        .cart_service
        .add_guest_item(
            &guest_id,
            AddItemRequest {
                variant_id: Uuid::now_v7(),
                quantity: 2,
                price: 9.99,
            },
        )
        .await
        .unwrap();

    env.state
        .cart_service
        .add_guest_item(
            &guest_id,
            AddItemRequest {
                variant_id: Uuid::now_v7(),
                quantity: 3,
                price: 10.5,
            },
        )
        .await
        .unwrap();

    let cart = env
        .state
        .cart_service
        .get_guest_cart(&guest_id)
        .await
        .unwrap();
    assert_eq!(cart.items.len(), 1);
    assert_eq!(cart.items[0].variant_id, Uuid::now_v7());
    assert_eq!(cart.items[0].quantity, 5);
    assert!((cart.items[0].price_at_add - 10.5).abs() < f64::EPSILON);

    env.state
        .cart_service
        .guest_repo
        .delete(&guest_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn add_guest_item_rejects_non_positive_quantity() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let guest_id = env.state.cart_service.create_guest_cart().await.unwrap();
    let err = env
        .state
        .cart_service
        .add_guest_item(
            &guest_id,
            AddItemRequest {
                variant_id: Uuid::now_v7(),
                quantity: 0,
                price: 1.0,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, CartError::InvalidQuantity));

    env.state
        .cart_service
        .guest_repo
        .delete(&guest_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn get_missing_guest_cart_returns_not_found() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let err = env
        .state
        .cart_service
        .get_guest_cart("does-not-exist-guest")
        .await
        .unwrap_err();
    assert!(matches!(err, CartError::NotFound));
}
