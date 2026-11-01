use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;


#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CartResponse {
    pub items: Vec<CartItem>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CartItem {
    pub variant_id: i64,
    pub quantity: i32,
    pub price_at_add: Option<f64>,
    pub added_at: DateTime<Utc>,
}