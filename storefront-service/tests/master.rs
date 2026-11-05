pub mod common;
pub mod handlers;

#[tokio::test]
async fn master() {
    let state = common::harness().await;
    handlers::auth_test::auth_master(state.clone()).await;
    handlers::checkout_tests::checkout_master(state.clone()).await;
    handlers::health_test::health_master(state.clone()).await;
    handlers::idempotency_test::idempotency_master(state.clone()).await;
    handlers::orders_test::order_master(state.clone()).await;
    handlers::payments_test::payment_master(state.clone()).await;
    handlers::problem_test::problem_master(state).await;
}
