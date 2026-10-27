use redis::aio::ConnectionManager;

pub async fn setup() -> ConnectionManager {
    let url = std::env::var("TEST_REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".into());
    let client = redis::Client::open(url).expect("open redis");
    ConnectionManager::new(client)
        .await
        .expect("redis manager (set TEST_REDIS_URL, needs Redis Stack)")
}

/// Helper to build unique keys for  tests...
pub fn uniq(prefix: &str) -> String {
    format!("test:{prefix}:{}", uuid::Uuid::now_v7())
}