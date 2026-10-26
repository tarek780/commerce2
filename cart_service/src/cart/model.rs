use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    pub sku_id: String,
    pub quantity: i32,
    pub price_at_add: f64,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCart {
    pub items: Vec<CartItem>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddItemRequest {
    pub sku_id: String,
    pub quantity: i32,
    pub price: f64,
}

#[derive(Debug, Deserialize)]
pub struct MergeRequest {
    pub guest_cart_id: String,
}

#[derive(Debug, Serialize)]
pub struct CartResponse {
    pub items: Vec<CartItem>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GuestCartCreated {
    pub guest_cart_id: String,
}
