mod config;
mod db;
mod error;
mod graphql;
mod grpc;
mod helpers;
mod outbox;
mod redis;
mod state;
mod stream_worker;
mod middleware;

//
pub mod pb_inv {
    tonic::include_proto!("inventory.v1");
}
//
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "category_service=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let cfg = config::AppConfig::from_env()?;
    let state = Arc::new(state::AppState::new(&cfg).await?);
    sqlx::migrate!("./migrations").run(&state.db).await?;
    tracing::info!("migrations applied");

    // bootstrap redis indexes
    redis::search::ensure_indexes(&mut state.redis.clone(), &cfg).await?;
    tracing::info!("redis indexes ensured");

    // stream consumer worker
    stream_worker::consumer::spawn(state.clone());

    // outbox publisher = late..
    //  tokio::spawn(outbox::publisher::spawn(state.clone()));

    // HTTP + GraphQL
    let app = graphql::router(state.clone());
    let addr: std::net::SocketAddr = cfg.bind_addr.parse()?;
    tracing::info!(%addr, "category-service listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
