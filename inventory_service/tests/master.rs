pub mod common;
pub mod handlers;

use crate::common::*;

#[tokio::test]
async fn master() {
    let env = build_env().await;
    //
    handlers::error_mapping::maps_domain_errors_to_status_codes();
    //
    handlers::expiry::expiry_worker_releases_and_emits_event(env.clone()).await;
    //
    handlers::grpc_service::grpc_commit_and_adjust(env.clone()).await;
    handlers::grpc_service::grpc_insufficient_stock_maps_to_failed_precondition(env.clone()).await;
    handlers::grpc_service::grpc_reserve_and_get_stock(env.clone()).await;
    handlers::grpc_service::grpc_unknown_variant_maps_to_not_found(env.clone()).await;
    //
    handlers::grpc_watch::watch_receives_reserved_event(env.clone()).await;
    //
    handlers::outbox::outbox_marks_events_as_published(env.clone()).await;
    //
    handlers::repo_commit::adjust_stock_success_and_negative_guard(env.clone()).await;
    handlers::repo_commit::commit_decrements_stock_and_reserved(env.clone()).await;
    handlers::repo_commit::commit_expired_reservation_fails(env.clone()).await;
    //
    handlers::repo_reserve::reserve_idempotency_mismatch_rejects(env.clone()).await;
    handlers::repo_reserve::reserve_idempotent_same_params_returns_same_id(env.clone()).await;
    handlers::repo_reserve::reserve_insufficient_stock(env.clone()).await;
    handlers::repo_reserve::reserve_release_and_available_counts(env.clone()).await;
    handlers::repo_reserve::reserve_unknown_variant(env).await;
}
