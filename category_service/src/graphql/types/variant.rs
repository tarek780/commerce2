use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use serde_json::Value;
use crate::db::impls::variant::VariantRow;

#[derive(SimpleObject)]
pub struct Variant {
    pub id: ID,
    pub product_id: ID,
    pub sku: String,
    pub price: String,                 // Decimal as String
    pub compare_at_price: Option<String>,
    pub image_url: Option<String>,
    pub attributes: Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<VariantRow> for Variant {
    fn from(r: VariantRow) -> Self {
        Self {
            id: ID(r.id.to_string()),
            product_id: ID(r.product_id.to_string()),
            sku: r.sku,
            price: r.price.to_string(),
            compare_at_price: r.compare_at_price.map(|x| x.to_string()),
            image_url: r.image_url,
            attributes: r.attributes,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}