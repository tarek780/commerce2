use crate::common;
use crate::common::TestEnv;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

fn get_base_url() -> String {
    std::env::var("TEST_APP_URL").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string())
}

pub async fn http_guest_flow_create_add_get(_env: &Arc<TestEnv>) {
    let client = Client::new();
    let base_url = get_base_url();

    // 1. Create: POST /api/guest/cart
    let res = client
        .post(format!("{base_url}/api/guest/cart"))
        .send()
        .await
        .expect("Failed to send POST /api/guest/cart");

    assert_eq!(res.status(), StatusCode::OK);

    let json: Value = res.json().await.expect("Failed to parse JSON response");
    let guest_id = json["guest_cart_id"]
        .as_str()
        .expect("guest_cart_id not found or not a string")
        .to_string();

    // 2. Add: POST /api/guest/cart/:id/items
    let uuid = Uuid::now_v7();
    let payload = cart_service::cart::model::AddItemRequest {
        variant_id: uuid,
        quantity: 1,
        price: 100.0,
    };

    let res = client
        .post(format!("{base_url}/api/guest/cart/{guest_id}/items"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST /api/guest/cart/:id/items");

    assert_eq!(res.status(), StatusCode::OK);

    // 3. Get: GET /api/guest/cart/:id
    let res = client
        .get(format!("{base_url}/api/guest/cart/{guest_id}"))
        .send()
        .await
        .expect("Failed to send GET /api/guest/cart/:id");

    assert_eq!(res.status(), StatusCode::OK);

    let json: Value = res.json().await.expect("Failed to parse JSON response");

    let variant = json["items"][0]["variant_id"].as_str();
    match variant {
        Some(e) => {
            assert_eq!(e, uuid.to_string().as_str());
        }
        None => {
            tracing::error!("json = null");
            return;
        }
    }
    assert_eq!(json["items"][0]["quantity"], 1);
}

pub async fn http_missing_guest_cart_returns_404(_env: &Arc<TestEnv>) {
    let client = Client::new();
    let base_url = get_base_url();

    let res = client
        .get(format!("{base_url}/api/guest/cart/nonexistent-id"))
        .send()
        .await
        .expect("Failed to send GET for nonexistent cart");

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

pub async fn c(_env: &Arc<TestEnv>) {
    let client = Client::new();
    let base_url = get_base_url();

    let res = client
        .post(format!("{base_url}/api/guest/cart"))
        .send()
        .await
        .expect("Failed to create guest cart");

    assert_eq!(res.status(), StatusCode::OK);
}

pub async fn http_master(env: Arc<TestEnv>) {
    http_guest_flow_create_add_get(&env).await;
    http_missing_guest_cart_returns_404(&env).await;
    c(&env).await;
}
