mod config;
mod error;
mod gateway;
mod outbox;
mod payment;
mod state;

pub mod pb {
    tonic::include_proto!("payment.v1");
}

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{extract::State, routing::post, Router};
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::PaymentConfig;
use crate::payment::service::PaymentSvc;
use crate::pb::payment_service_server::PaymentServiceServer;
use crate::state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "payment_service=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();
    dotenvy::dotenv().ok();

    let cfg = PaymentConfig::from_env().await?;
    let state = Arc::new(AppState::new(cfg.clone()).await?);
    sqlx::migrate!("./migrations").run(&state.db).await?;
    tracing::info!("migrations applied");

    // HTTP (webhooks)
    {
        let st = state.clone();
        let http_addr: SocketAddr = cfg.http_addr.parse()?;
        let app = Router::new()
            .route(
                "/webhooks/payment/:payment_id",
                post(
                    |state: State<Arc<AppState>>,
                     path: axum::extract::Path<String>,
                     headers: axum::http::HeaderMap,
                     body: axum::body::Bytes| async move {
                        payment::webhook::handle(state, path, headers, body).await
                    },
                ),
            )
            .with_state(st.clone());
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(http_addr).await.unwrap();
            tracing::info!(%http_addr, "payment http listening");
            axum::serve(listener, app).await.unwrap();
        });
    }

    // Expiry worker
    {
        let st = state.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(Duration::from_secs(st.config.expiry_scan_secs));
            loop {
                iv.tick().await;
                let _ = payment::expiry::expire_batch(&st).await;
            }
        });
    }

    // Outbox → Kafka
    {
        let st = state.clone();
        let topic = st.config.kafka_payment_topic.clone();
        tokio::spawn(async move {
            let mut iv = tokio::time::interval(Duration::from_secs(1));
            loop {
                iv.tick().await;
                if let Err(e) = outbox::publisher::publish_batch(&st.db, &topic).await {
                    tracing::error!(?e, "outbox failed");
                }
            }
        });
    }

    let addr: SocketAddr = cfg.grpc_addr.parse()?;
    tracing::info!(%addr, "payment gRPC listening");
    Server::builder()
        .add_service(PaymentServiceServer::new(PaymentSvc::new(state)))
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
