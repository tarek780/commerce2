mod clients;
mod config;
mod dto;
mod error;
mod handlers;
mod helpers;
mod middleware;
mod openapi;
mod problem;
mod routes;
mod state;

use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "storefront_service=debug,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let cfg = config::AppConfig::from_env()?;
    tracing::info!(?cfg, "storefront-service booting");

    let state = Arc::new(state::AppState::new(&cfg).await?);
    tracing::info!("upstream clients initialized");

    let app = routes::build(state.clone());
    let addr: std::net::SocketAddr = cfg.http_addr.parse()?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "storefront-service listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok(); };
    #[cfg(unix)]
    let term = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap().recv().await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}