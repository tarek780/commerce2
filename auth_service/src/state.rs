use crate::config::Config;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub config: Arc<Config>,
    pub mailer: Arc<AsyncSmtpTransport<Tokio1Executor>>,
}

impl AppState {
    pub fn new_for_tests(pool: PgPool, redis: ConnectionManager) -> Self {

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
            .port(25)
            .credentials(Credentials::new("x".into(), "y".into()))
            .build();

        Self {
            pool,
            redis,
            config: Arc::new(Config {
                bind_address: "127.0.0.1:0".into(),
                database_url: String::new(),
                redis_url: String::new(),

                secret_key: "test-secret-key-32-chars-minimum-xx".into(),
                jwt_access_expiry_secs: 3600,
                jwt_refresh_expiry_secs: 86400,
                jwt_issuer: "test".into(),
                jwt_audience: "test".into(),

                argon2_memory_cost: 4096,
                argon2_time_cost: 1,
                argon2_parallelism: 1,

                email_verification_expiry_secs: 600,
                email_verify_rate_limit_secs: 60,

                smtp_host: "127.0.0.1".into(),
                smtp_port: 25,
                smtp_username: "x".into(),
                smtp_password: "y".into(),
                smtp_from: "noreply@test.local".into(),
                app_public_url: "http://test.local".into(),
            }),
            mailer: Arc::new(mailer),
        }
    }
}