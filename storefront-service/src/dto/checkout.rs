use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CheckoutResponse {
    pub order_id: String,
    pub status: String,
    pub total_amount: String,
    pub currency: String,
    pub payment_id: String,
    pub payment_url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}