pub mod common;
pub mod handlers;

pub async fn master() {
    let state = common::test_state().await;
    //
    handlers::config_tests::config_master().await;
    //
    handlers::expiry_tests::expiry_master(state.clone()).await;
    //
    handlers::gateway_tests::gateway_master().await;
    //
    handlers::repo_tests::repo_master(state.clone()).await;
    //
    handlers::service_tests::service_master(state.clone()).await;
    //
    handlers::webhook_tests::webhook_master(state).await;
}
