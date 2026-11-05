pub mod common;
pub mod handlers;

//
#[tokio::test]
async fn master() {
    let pool = common::db::build_state().await;
    let client = common::redis::setup().await;

    //
    handlers::integration_category_repo::children_and_limits(pool.clone()).await;
    handlers::integration_category_repo::crud_flow(pool.clone()).await;
    //
    handlers::integration_product_repo::crud_and_get_by_slug(pool.clone()).await;
    handlers::integration_product_repo::filters_and_cursor_pagination(pool.clone()).await;
    //
    handlers::integration_redis_json::get_missing_returns_none(client.1.clone()).await;
    handlers::integration_redis_json::set_get_del_roundtrip(client.1.clone()).await;
    //
    handlers::integration_redis_search::search_by_category_brand_active_and_price(client.1.clone())
        .await;
    //
    handlers::integration_redis_stream::consumer_group_roundtrip(client.1.clone()).await;
    handlers::integration_redis_stream::publish_appends_to_stream(client.1).await;
    //
    handlers::integration_variant_repo::cascade_delete_on_product_removal(pool.clone()).await;
    handlers::integration_variant_repo::insert_list_delete_and_sku_conflict(pool).await;
    //
    handlers::unit_config::defaults_applied();
    handlers::unit_config::missing_required_returns_config_error();
    handlers::unit_config::overrides_applied();
    //
    handlers::unit_cursor::decode_required_error_variant();
    handlers::unit_cursor::decode_required_ok();
    handlers::unit_cursor::invalid_inputs_yield_none();
    handlers::unit_cursor::roundtrip();
    handlers::unit_cursor::url_safe_alphabet();
    //
    handlers::unit_error::display_messages();
    handlers::unit_error::from_sqlx_error();
    handlers::unit_error::graphql_extend_wraps_message();
    handlers::unit_error::grpc_status_from_app_error();
    //
    handlers::unit_pagination::cursor_from_after();
    handlers::unit_pagination::default_limit_is_20();
    handlers::unit_pagination::filter_default_is_all_none();
    handlers::unit_pagination::invalid_after_yields_none();
    handlers::unit_pagination::limit_is_clamped();
    //
    handlers::unit_variant_type::from_row_handles_optionals_none();
    handlers::unit_variant_type::from_row_maps_all_fields();
    //
    handlers::unit_stream_events::ids();
    handlers::unit_stream_events::kinds();
    handlers::unit_stream_events::serde_tagged_roundtrip();
}
