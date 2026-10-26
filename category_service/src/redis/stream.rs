use crate::error::AppResult;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogEvent {
    CategoryCreated { id: i64, payload: Value },
    CategoryUpdated { id: i64, payload: Value },
    CategoryDeleted { id: i64 },
    ProductCreated { id: i64, payload: Value },
    ProductUpdated { id: i64, payload: Value },
    ProductDeleted { id: i64 },
}

impl CatalogEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CategoryCreated { .. } => "category_created",
            Self::CategoryUpdated { .. } => "category_updated",
            Self::CategoryDeleted { .. } => "category_deleted",
            Self::ProductCreated { .. } => "product_created",
            Self::ProductUpdated { .. } => "product_updated",
            Self::ProductDeleted { .. } => "product_deleted",
        }
    }
    pub fn id(&self) -> i64 {
        match self {
            Self::CategoryCreated { id, .. }
            | Self::CategoryUpdated { id, .. }
            | Self::CategoryDeleted { id }
            | Self::ProductCreated { id, .. }
            | Self::ProductUpdated { id, .. }
            | Self::ProductDeleted { id } => *id,
        }
    }
}

pub async fn publish(
    mgr: &mut ConnectionManager,
    stream: &str,
    event: &CatalogEvent,
) -> AppResult<String> {
    let payload = serde_json::to_string(event)?;
    let id: String = redis::cmd("XADD")
        .arg(stream)
        .arg("*")
        .arg("kind")
        .arg(event.kind())
        .arg("id")
        .arg(event.id())
        .arg("payload")
        .arg(payload)
        .query_async(mgr)
        .await?;
    Ok(id)
}
