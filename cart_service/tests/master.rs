pub mod common;
pub mod handlers;

use crate::common::*;

#[tokio::test]
async fn master() {
    let env = common::build_state().await;

    //
    handlers::guest_cart::add_guest_item_rejects_non_positive_quantity(env).await;
    handlers::guest_cart::create_get_and_add_items_to_guest_cart(env).await;
    handlers::guest_cart::get_missing_guest_cart_returns_not_found(env).await;
    //
    handlers::http_guest::http_guest_flow_create_add_get(env).await;
    handlers::http_guest::http_missing_guest_cart_returns_404(env).await;
    handlers::http_guest::c(env).await;
    //
    handlers::merge_cart::merge_empty_guest_cart_just_deletes_it(env).await;
    handlers::merge_cart::merge_guest_into_user_combines_and_deletes_guest(env).await;
    //
    handlers::user_cart::bulk_add_and_validation(env).await;
    handlers::user_cart::user_cart_roundtrip_and_cache_invalidation(env).await;
}
