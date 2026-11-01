mod common;

use cart_service::cart::model::AddItemRequest;
use uuid::Uuid;
#[tokio::test]
async fn merge_guest_into_user_combines_and_deletes_guest() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let user_id = common::unique_user_id();

    env.state
        .cart_service
        .add_user_item(
            &user_id,
            AddItemRequest {
                variant_id: Uuid::now_v7(),
                quantity: 1,
                price: 5.0,
            },
        )
        .await
        .unwrap();

    let guest_id = env.state.cart_service.create_guest_cart().await.unwrap();
    env.state
        .cart_service
        .add_guest_item(
            &guest_id,
            AddItemRequest {
                variant_id: Uuid::now_v7(),
                quantity: 2,
                price: 6.0,
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
                price: 2.0,
            },
        )
        .await
        .unwrap();

    env.state
        .cart_service
        .merge_guest_into_user(&user_id, &guest_id)
        .await
        .unwrap();

    let cart = env
        .state
        .cart_service
        .get_user_cart(&user_id)
        .await
        .unwrap();
    assert_eq!(cart.items.len(), 2);
    let a = cart
        .items
        .iter()
        .find(|i| i.variant_id == Uuid::now_v7())
        .unwrap();
    let z = cart
        .items
        .iter()
        .find(|i| i.variant_id == Uuid::now_v7())
        .unwrap();
    assert_eq!(a.quantity, 3);
    assert_eq!(z.quantity, 3);

    let err = env
        .state
        .cart_service
        .get_guest_cart(&guest_id)
        .await
        .unwrap_err();
    assert!(matches!(err, cart_service::error::CartError::NotFound));
}

#[tokio::test]
async fn merge_empty_guest_cart_just_deletes_it() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let user_id = common::unique_user_id();
    let guest_id = env.state.cart_service.create_guest_cart().await.unwrap();

    env.state
        .cart_service
        .merge_guest_into_user(&user_id, &guest_id)
        .await
        .unwrap();

    let err = env
        .state
        .cart_service
        .get_guest_cart(&guest_id)
        .await
        .unwrap_err();
    assert!(matches!(err, cart_service::error::CartError::NotFound));
}
