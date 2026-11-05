pub mod common;
pub mod handlers;

#[tokio::test]
async fn master() {
    let env = common::build_state().await;

    //
    handlers::guest_cart::guest_master(env.clone()).await;
    //
    handlers::http_guest::http_master(env.clone()).await;
    //
    handlers::merge_cart::merge_master(env.clone()).await;
    //
    handlers::user_cart::user_master(env).await;
}
