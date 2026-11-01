mod error;
mod inventory;
mod outbox;

pub mod pb {
    tonic::include_proto!("inventory.v1");
}

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::inventory::repo::InventoryRepo;
use crate::inventory::service::InventorySvc;
use crate::pb::inventory_service_server::InventoryServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "inventory_service=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("migrations applied");

    let (stock_tx, _) = broadcast::channel(1024);

    let repo = Arc::new(InventoryRepo::new(pool.clone()));
    let svc = InventorySvc::new(repo.clone(), stock_tx.clone());

    // Expiry worker
    {
        let pool = pool.clone();
        let tx = stock_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                match inventory::expiry::expire_batch(&pool, &tx).await {
                    Ok(n) if n > 0 => tracing::info!(expired = n, "expired reservations released"),
                    Err(e) => tracing::error!(?e, "expiry failed"),
                    _ => {}
                }
            }
        });
    }

    // Outbox worker
    {
        let pool = pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                if let Err(e) = outbox::publisher::publish_batch(&pool).await {
                    tracing::error!(?e, "outbox failed");
                }
            }
        });
    }

    let addr: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;

    tracing::info!(%addr, "starting gRPC");
    Server::builder()
        .add_service(InventoryServiceServer::new(svc))
        .serve_with_shutdown(addr, shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let term = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .unwrap()
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = term => {} }
}
