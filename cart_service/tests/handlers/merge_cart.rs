use crate::common;
use crate::common::TestEnv;
use cart_service::cart::model::AddItemRequest;
use std::sync::Arc;
use uuid::Uuid;

pub async fn merge_guest_into_user_combines_and_deletes_guest(env: &Arc<TestEnv>) {
    let user_id = common::unique_user_id();

    match env
        .state
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
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    }

    let guest_id = match env.state.cart_service.create_guest_cart().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    match env
        .state
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
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    match env
        .state
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
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    }

    match env
        .state
        .cart_service
        .merge_guest_into_user(&user_id, &guest_id)
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
    assert_eq!(cart.items.len(), 2);
    match cart.items.iter().find(|i| i.variant_id == Uuid::now_v7()) {
        Some(a) => {
            assert_eq!(a.quantity, 3);
        }
        None => {
            eprintln!("NULL");
            return;
        }
    }
    match cart.items.iter().find(|i| i.variant_id == Uuid::now_v7()) {
        Some(z) => {
            assert_eq!(z.quantity, 3);
        }
        None => {
            eprintln!("NULL");
            return;
        }
    }

    let err = env
        .state
        .cart_service
        .get_guest_cart(&guest_id)
        .await
        .unwrap_err();
    assert!(matches!(err, cart_service::error::CartError::NotFound));
}

pub async fn merge_empty_guest_cart_just_deletes_it(env: &Arc<TestEnv>) {
    let user_id = common::unique_user_id();
    let guest_id = match env.state.cart_service.create_guest_cart().await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    match env
        .state
        .cart_service
        .merge_guest_into_user(&user_id, &guest_id)
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    }

    let err = env
        .state
        .cart_service
        .get_guest_cart(&guest_id)
        .await
        .unwrap_err();
    assert!(matches!(err, cart_service::error::CartError::NotFound));
}

pub async fn merge_master(env: Arc<TestEnv>) {
    merge_empty_guest_cart_just_deletes_it(&env).await;
    merge_guest_into_user_combines_and_deletes_guest(&env).await;
}
