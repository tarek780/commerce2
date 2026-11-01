#![allow(dead_code)]

use std::sync::Arc;
use tokio::io::stdout;

use axum::Router;
use tower_http::trace::TraceLayer;

use cart_service::cart::repository::{GuestCartRepo, UserCartRepo};
use cart_service::cart::service::CartService;
use cart_service::state::AppState;

use cart_service::auth::layer::AuthLayer;
use cart_service::cart::handlers;

use sqlx::postgres::PgPoolOptions;
use std::io::Stdout;
use testcontainers::{
    ContainerAsync,
    core::{Image, ImageExt, IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};
use testcontainers::{ContainerRequest, GenericImage};
use tokio::io::AsyncWriteExt;

pub struct TestEnv {
    pub state: Arc<AppState>,
}

pub async fn build_state() -> Option<TestEnv> {
    //
    unsafe {
        std::env::set_var("SECRET_KEY", "test-secret-key-32-chars-minimum-xx");
        std::env::set_var("RUST_LOG", "info");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let pg_image = GenericImage::new("postgres", "16.15-alpine3.24")
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_USER", "myuser")
        .with_env_var("POSTGRES_PASSWORD", "mypassword")
        .with_env_var("POSTGRES_DB", "mydb");

    let pg_container = pg_image
        .start()
        .await
        .expect("Failed to start Postgres container");

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let pg_port = match pg_container.get_host_port_ipv4(5432).await {
        Ok(c) => c,
        Err(e) => {
            let _ = stdout()
                .write_all(format!("Failed to get Postgres container port: {}\n", e).as_bytes())
                .await;
            panic!("Failed to get Postgres container port: {}", e);
        }
    };

    let database_url =
        format!("postgres://myuser:mypassword@127.0.0.1:{pg_port}/mydb?sslmode=disable");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect test postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let redis_image = GenericImage::new("redis", "alpine3.23")
        .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"));

    let redis_container = redis_image
        .start()
        .await
        .expect("Failed to start Redis container");

    let redis_host = redis_container.get_host().await.unwrap();
    let redis_port = match redis_container.get_host_port_ipv4(6379).await {
        Ok(c) => c,
        Err(e) => {
            let _ = stdout()
                .write_all(format!("Failed to get Redis container port: {}\n", e).as_bytes())
                .await;
            panic!("Failed to get Redis container port: {}", e);
        }
    };
    let redis_url = format!("redis://{redis_host}:{redis_port}");

    let redis_client = redis::Client::open(redis_url).expect("redis client");
    let redis = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("redis connection");
    //

    //
    let guest_repo = GuestCartRepo {
        redis: redis.clone(),
    };

    let user_repo = UserCartRepo { pool, redis };
    let cart_service = Arc::new(CartService {
        guest_repo,
        user_repo,
    });
    let state = Arc::new(AppState { cart_service });

    Some(TestEnv { state })
}

pub async fn cleanup_redis(redis_url: &str, keys: &[&str]) {
    if let Ok(client) = redis::Client::open(redis_url) {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            for k in keys {
                let _: Result<i64, _> = redis::cmd("DEL").arg(k).query_async(&mut conn).await;
            }
        }
    }
}

pub fn unique_user_id() -> String {
    format!("test-user-{}", uuid::Uuid::now_v7())
}

pub fn unique_guest_id() -> String {
    format!("test-guest-{}", uuid::Uuid::now_v7())
}
//

pub fn build_router(state: Arc<AppState>) -> Router {
    let guest_routes = Router::new()
        .route(
            "/guest/cart",
            axum::routing::post(handlers::create_guest_cart),
        )
        .route(
            "/guest/cart/:guest_id",
            axum::routing::get(handlers::get_guest_cart),
        )
        .route(
            "/guest/cart/:guest_id/items",
            axum::routing::post(handlers::add_guest_item),
        );

    let user_routes = Router::new()
        .route("/cart", axum::routing::get(handlers::get_user_cart))
        .route("/cart/items", axum::routing::post(handlers::add_user_item))
        .route(
            "/cart/items/bulk",
            axum::routing::post(handlers::add_user_items_bulk),
        )
        .route("/cart/merge", axum::routing::post(handlers::merge_cart))
        .layer(AuthLayer {});

    Router::new()
        .nest("/api", guest_routes.merge(user_routes))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
