pub mod common;
pub mod handlers;
//
use crate::common::*;

#[tokio::test]
async fn master() {
    let state = setup_test_state().await;
    //
    handlers::expiry_tests::expire_batch_marks_due_orders_cancelled(state.clone()).await;
    handlers::expiry_tests::expire_batch_skips_not_due(state.clone()).await;
    //
    handlers::orchestrator_tests::cancel_final_state_rejected(state.clone()).await;
    handlers::orchestrator_tests::cancel_pending_succeeds_without_clients(state.clone()).await;
    handlers::orchestrator_tests::empty_items_rejected(state.clone()).await;
    handlers::orchestrator_tests::happy_path_full_orchestration(state.clone()).await;
    //
    handlers::outbox_tests::publish_batch_empty_returns_zero(state.clone()).await;
    handlers::outbox_tests::publish_batch_marks_events_published(state.clone()).await;
    //
    handlers::repo_tests::duplicate_idempotency_key_returns_duplicate(state.clone()).await;
    handlers::repo_tests::get_missing_returns_not_found(state.clone()).await;
    handlers::repo_tests::insert_and_get_order(state.clone()).await;
    handlers::repo_tests::list_expired_returns_only_due_orders(state.clone()).await;
    handlers::repo_tests::transition_from_wrong_state_fails(state.clone()).await;
    handlers::repo_tests::transition_pending_to_reserved(state.clone()).await;
    //
    handlers::service_tests::cancel_order_final_state_returns_failed_precondition(state.clone())
        .await;
    handlers::service_tests::cancel_order_pending_success(state.clone()).await;
    handlers::service_tests::create_order_bad_price_invalid_argument(state.clone()).await;
    handlers::service_tests::create_order_empty_items_invalid_argument(state.clone()).await;
    handlers::service_tests::get_order_not_found_returns_status(state.clone()).await;
    handlers::service_tests::get_order_returns_fields(state).await;
}
