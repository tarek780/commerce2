use std::process::id;

use crate::error::AppResult;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CatalogEvent {
    CategoryCreated { id: Uuid, payload: Value },
    CategoryUpdated { id: Uuid, payload: Value },
    CategoryDeleted { id: Uuid },
    ProductCreated { id: Uuid, payload: Value },
    ProductUpdated { id: Uuid, payload: Value },
    ProductDeleted { id: Uuid },
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
    pub fn id(&self) -> Uuid {
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
    let idd = String::from(event.id());
    let id: String = redis::cmd("XADD")
        .arg(stream)
        .arg("*")
        .arg("kind")
        .arg(event.kind())
        .arg("id")
        .arg(idd)
        .arg("payload")
        .arg(payload)
        .query_async(mgr)
        .await?;
    Ok(id)
}
