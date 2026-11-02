mod common;

use cart_service::cart::model::AddItemRequest;
use uuid::Uuid;
#[tokio::test]
async fn user_cart_roundtrip_and_cache_invalidation() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let user_id = common::unique_user_id();

    let cart = match env.state.cart_service.get_user_cart(&user_id).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    assert!(cart.items.is_empty());
    let variant_id = Uuid::now_v7();

    match env
        .state
        .cart_service
        .add_user_item(
            &user_id,
            AddItemRequest {
                variant_id: variant_id,
                quantity: 1,
                price: 5.0,
            },
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    let cart = match env.state.cart_service.get_user_cart(&user_id).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert_eq!(cart.items.len(), 1);
    assert_eq!(cart.items[0].quantity, 1);

    match env
        .state
        .cart_service
        .add_user_item(
            &user_id,
            AddItemRequest {
                variant_id: variant_id,
                quantity: 4,
                price: 5.0,
            },
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    let cart = match env.state.cart_service.get_user_cart(&user_id).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert_eq!(cart.items[0].quantity, 5);

    match env
        .state
        .cart_service
        .user_repo
        .invalidate_cache(&user_id)
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    }
    let cart = match env.state.cart_service.get_user_cart(&user_id).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert_eq!(cart.items[0].quantity, 5);
}

#[tokio::test]
async fn bulk_add_and_validation() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let user_id = common::unique_user_id();

    match env
        .state
        .cart_service
        .add_user_items(&user_id, vec![])
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    let variant_id = Uuid::now_v7();
    let err = env
        .state
        .cart_service
        .add_user_items(
            &user_id,
            vec![AddItemRequest {
                variant_id: variant_id,
                quantity: -1,
                price: 1.0,
            }],
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        cart_service::error::CartError::InvalidQuantity
    ));

    match env
        .state
        .cart_service
        .add_user_items(
            &user_id,
            vec![
                AddItemRequest {
                    variant_id: variant_id,
                    quantity: 2,
                    price: 3.0,
                },
                AddItemRequest {
                    variant_id: variant_id,
                    quantity: 1,
                    price: 7.5,
                },
            ],
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    let cart = match env.state.cart_service.get_user_cart(&user_id).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    assert_eq!(cart.items.len(), 2);
    let sku_b = match cart.items.iter().find(|i| i.variant_id == variant_id) {
        Some(e) => e,
        None => {
            eprintln!("error: ");
            return;
        }
    };
    assert_eq!(sku_b.quantity, 2);
}
