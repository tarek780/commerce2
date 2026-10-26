// src/main.rs
mod auth;
mod cart;
mod error;
mod state;

use axum::{
    Router,
    routing::{get, post},
};
use std::env;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::prelude::*;

use crate::{
    auth::layer::AuthLayer,
    cart::repository::{GuestCartRepo, UserCartRepo},
    cart::service::CartService,
    state::AppState,
};

#[tokio::main]
async fn main() -> Result<(), error::CartError> {
    dotenvy::dotenv().ok();
    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::EnvFilter::new(
    //         std::env::var("RUST_LOG")
    //             .unwrap_or_else(|_| "cart_service=debug,tower_http=info".into()),
    //     ))
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "cart_service=debug,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool_env = env::var("DATABASE_URL")?;
    let redis_env = env::var("REDIS_URL")?;

    let pool = sqlx::PgPool::connect(pool_env.as_str()).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let redis_client = redis::Client::open(redis_env)?;
    let redis = redis_client.get_multiplexed_async_connection().await?;
    let guest_repo = GuestCartRepo {
        redis: redis.clone(),
    };
    let user_repo = UserCartRepo { pool, redis };
    let cart_service = Arc::new(CartService {
        guest_repo,
        user_repo,
    });
    let state = Arc::new(AppState { cart_service });

    let guest_routes = Router::new()
        .route("/guest/cart", post(cart::handlers::create_guest_cart))
        .route("/guest/cart/:guest_id", get(cart::handlers::get_guest_cart))
        .route(
            "/guest/cart/:guest_id/items",
            post(cart::handlers::add_guest_item),
        );

    let user_routes = Router::new()
        .route("/cart", get(cart::handlers::get_user_cart))
        .route("/cart/items", post(cart::handlers::add_user_item))
        .route(
            "/cart/items/bulk",
            post(cart::handlers::add_user_items_bulk),
        )
        .route("/cart/merge", post(cart::handlers::merge_cart))
        .layer(AuthLayer {});

    let app = Router::new()
        .nest("/api", guest_routes.merge(user_routes))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind = std::env::var("BIND_ADDRESS")?;

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("cart service listening on 0.0.0.0:3000");
    axum::serve(listener, app).await?;
    Ok(())
}
