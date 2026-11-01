use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderDto {
    pub id: String,
    pub customer_id: String,
    pub status: String,
    pub total_amount: String,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderEdge {
    pub cursor: String,
    pub node: OrderDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderConnection {
    pub edges: Vec<OrderEdge>,
    pub page_info: PageInfoDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PageInfoDto {
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PayOrderResponse {
    pub order_id: String,
    pub payment_id: String,
    pub payment_url: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CancelOrderBody {
    pub reason: String,
}