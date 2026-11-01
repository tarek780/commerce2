mod clients;
mod config;
mod error;
mod kafka;
mod order;
mod outbox;
mod state;

pub mod pb_order {
    tonic::include_proto!("order.v1");
}
pub mod pb_inv {
    tonic::include_proto!("inventory.v1");
}
pub mod pb_pay {
    tonic::include_proto!("payment.v1");
}

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tonic::transport::Server;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::OrderConfig;
use crate::pb_order::order_service_server::OrderServiceServer;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "order_service=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
    dotenvy::dotenv().ok();

    let cfg = OrderConfig::from_env()?;
    let state = Arc::new(AppState::new(cfg.clone()).await?);

    sqlx::migrate!("./migrations").run(&state.db).await?;
    tracing::info!("migrations applied");

    // Expiry worker
    {
        let st = state.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(Duration::from_secs(st.config.expiry_scan_secs));
            loop {
                iv.tick().await;
                match order::expiry::expire_batch(&st).await {
                    Ok(n) if n > 0 => tracing::info!(expired = n, "orders expired"),
                    Err(e) => tracing::error!(?e, "order expiry failed"),
                    _ => {}
                }
            }
        });
    }

    // Outbox worker
    {
        let st = state.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(Duration::from_secs(1));
            loop {
                iv.tick().await;
                if let Err(e) = outbox::publisher::publish_batch(&st.db).await {
                    tracing::error!(?e, "outbox failed");
                }
            }
        });
    }

    // Kafka consumer
    kafka::consumer::spawn(state.clone());

    let addr: SocketAddr = cfg.bind_addr.parse()?;
    tracing::info!(%addr, "order gRPC listening");
    Server::builder()
        .add_service(OrderServiceServer::new(order::service::OrderSvc::new(
            state,
        )))
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
