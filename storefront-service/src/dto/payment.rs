use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaymentDto {
    pub id: String,
    pub order_id: String,
    pub status: String,
    pub amount: String,
    pub currency: String,
    pub gateway_reference: String,
    pub payment_url: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}