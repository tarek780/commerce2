use chrono::Utc;
use storefront_service::clients::order::pb_order::{OrderResponse, PayOrderResponse};
use storefront_service::clients::payment::pb_payment::PaymentResponse;
use storefront_service::dto::cart::{CartItem, CartResponse};
use uuid::Uuid;

pub fn cart_item(variant_id: Uuid, quantity: i32) -> CartItem {
    CartItem {
        variant_id,
        quantity,
        price_at_add: Some(19.99),
        added_at: Utc::now(),
    }
}

pub fn cart_response(items: Vec<CartItem>) -> CartResponse {
    CartResponse {
        items,
        updated_at: Utc::now(),
    }
}

pub fn order_response(id: &str, customer_id: &str) -> OrderResponse {
    OrderResponse {
        id: id.into(),
        customer_id: customer_id.into(),
        status: "awaiting_payment".into(),
        total_amount: "60.00".into(),
        currency: "USD".into(),
        reservation_id: "res-1".into(),
        payment_id: "pay-1".into(),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        failure_reason: None,
    }
}

pub fn pay_response(order_id: &str) -> PayOrderResponse {
    PayOrderResponse {
        order_id: order_id.into(),
        payment_id: "pay-1".into(),
        payment_url: "http://pay/mock-1".into(),
        status: "processing".into(),
    }
}

pub fn payment_response(id: &str) -> PaymentResponse {
    PaymentResponse {
        id: id.into(),
        order_id: "order-1".into(),
        status: "succeeded".into(),
        amount: "60.00".into(),
        currency: "USD".into(),
        gateway_reference: "mock_ref".into(),
        payment_url: "http://pay/mock-1".into(),
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        failure_reason: None,
    }
}
