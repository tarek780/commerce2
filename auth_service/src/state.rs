use crate::config::Config;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub config: Arc<Config>,
    pub mailer: Arc<lettre::AsyncSmtpTransport<lettre::Tokio1Executor>>,
}
