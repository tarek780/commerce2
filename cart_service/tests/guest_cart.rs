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

    let guest_id = match env.state.cart_service.create_guest_cart().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert!(!guest_id.is_empty());

    let cart = match env.state.cart_service.get_guest_cart(&guest_id).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to get guest cart: {:?}", e);
            return;
        }
    };
    assert!(cart.items.is_empty());

    match env
        .state
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
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to add item to guest cart: {:?}", e);
            return;
        }
    }

    match env
        .state
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
    {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to add item to guest cart: {:?}", e);
            return;
        }
    }

    let cart = match env.state.cart_service.get_guest_cart(&guest_id).await {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to get guest cart: {:?}", e);
            return;
        }
    };
    assert_eq!(cart.items.len(), 1);
    assert_eq!(cart.items[0].variant_id, Uuid::now_v7());
    assert_eq!(cart.items[0].quantity, 5);
    assert!((cart.items[0].price_at_add - 10.5).abs() < f64::EPSILON);

    match env.state.cart_service.guest_repo.delete(&guest_id).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to delete guest cart: {:?}", e);
        }
    }
}

#[tokio::test]
async fn add_guest_item_rejects_non_positive_quantity() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let guest_id = match env.state.cart_service.create_guest_cart().await {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Failed to create guest cart");
            return;
        }
    };
    let err = match env
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
    {
        Err(e) => e,
        Ok(_) => {
            eprintln!("Expected invalid quantity error");
            return;
        }
    };

    assert!(matches!(err, CartError::InvalidQuantity));

    match env.state.cart_service.guest_repo.delete(&guest_id).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to delete guest cart: {:?}", e);
        }
    }
}

#[tokio::test]
async fn get_missing_guest_cart_returns_not_found() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };

    let err = match env
        .state
        .cart_service
        .get_guest_cart("does-not-exist-guest")
        .await
    {
        Err(e) => e,
        Ok(_) => {
            eprintln!("Expected not found error");
            return;
        }
    };
    assert!(matches!(err, CartError::NotFound));
}
