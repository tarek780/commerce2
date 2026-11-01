use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{AppError, AppResult};

#[async_trait]
pub trait IdempotencyStore: Send + Sync + 'static {
    /// None = new key reserved, Some(body) = duplicate
    async fn try_reserve(&self, key: &str) -> AppResult<Option<String>>;
    async fn store_result(&self, key: &str, body: &str) -> AppResult<()>;
}

pub fn key_from_header_or_new(h: Option<&str>) -> (String, bool) {
    match h {
        Some(s) if !s.is_empty() => (s.to_string(), true),
        _ => (Uuid::new_v4().to_string(), false),
    }
}

// Redis
pub struct RedisIdempotencyStore {
    redis: ConnectionManager,
    ttl_secs: u64,
}

impl RedisIdempotencyStore {
    pub fn new(redis: ConnectionManager, ttl_secs: u64) -> Self {
        Self { redis, ttl_secs }
    }
}

#[async_trait]
impl IdempotencyStore for RedisIdempotencyStore {
    async fn try_reserve(&self, key: &str) -> AppResult<Option<String>> {
        let mut redis = self.redis.clone();
        let full = format!("idem:{key}");
        let set: Option<String> = redis::cmd("SET")
            .arg(&full).arg("__PENDING__")
            .arg("NX").arg("EX").arg(self.ttl_secs)
            .query_async(&mut redis).await?;

        if set.is_some() { return Ok(None); }

        let existing: Option<String> = redis.get(&full).await?;
        match existing.as_deref() {
            Some("__PENDING__") => Err(AppError::IdempotencyConflict),
            Some(body) => Ok(Some(body.to_string())),
            None => Ok(None),
        }
    }

    async fn store_result(&self, key: &str, body: &str) -> AppResult<()> {
        let mut redis = self.redis.clone();
        let full = format!("idem:{key}");
        let _: () = redis::cmd("SET")
            .arg(&full).arg(body)
            .arg("EX").arg(self.ttl_secs)
            .query_async(&mut redis).await?;
        Ok(())
    }
}

// ===== In-memory (tests) =====
pub struct MemoryIdempotencyStore {
    map: Arc<Mutex<HashMap<String, String>>>,
}

impl MemoryIdempotencyStore {
    pub fn new() -> Self {
        Self { map: Arc::new(Mutex::new(HashMap::new())) }
    }
}

#[async_trait]
impl IdempotencyStore for MemoryIdempotencyStore {
    async fn try_reserve(&self, key: &str) -> AppResult<Option<String>> {
        let mut m = self.map.lock().await;
        if let Some(v) = m.get(key) {
            if v == "__PENDING__" {
                return Err(AppError::IdempotencyConflict);
            }
            return Ok(Some(v.clone()));
        }
        m.insert(key.to_string(), "__PENDING__".into());
        Ok(None)
    }
    async fn store_result(&self, key: &str, body: &str) -> AppResult<()> {
        self.map.lock().await.insert(key.to_string(), body.to_string());
        Ok(())
    }
}