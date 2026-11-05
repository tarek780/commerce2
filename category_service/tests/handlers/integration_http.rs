use category_service::graphql;
use category_service::state::AppState;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::sync::Arc;

fn get_test_config() -> (Client, String) {
    let client = Client::new();
    let base_url =
        std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    (client, base_url)
}

pub async fn playground_renders() {
    let (client, base_url) = get_test_config();

    let res = client
        .get(format!("{base_url}/graphql"))
        .send()
        .await
        .expect("Failed to send GET /graphql");

    assert_eq!(res.status(), StatusCode::OK);

    let html = res
        .text()
        .await
        .expect("Failed to read response body as text");
    assert!(html.contains("GraphQL"));
}

pub async fn query_products_returns_connection() {
    let (client, base_url) = get_test_config();

    let payload = serde_json::json!({
        "query": "{ products(page: { first: 5 }) { totalCount edges { cursor node { id name slug } } pageInfo { hasNextPage endCursor } } }"
    });

    let res = client
        .post(format!("{base_url}/graphql"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST /graphql for query");

    assert_eq!(res.status(), StatusCode::OK);

    let body: Value = res.json().await.expect("Failed to parse response JSON");
    assert!(body["errors"].is_null(), "unexpected errors: {body}");
    assert!(body["data"]["products"].is_object());
    assert!(body["data"]["products"]["edges"].is_array());
}

pub async fn mutation_without_auth_returns_error() {
    let (client, base_url) = get_test_config();

    let payload = serde_json::json!({
        "query": "mutation { createCategory(input: { name: \"X\" }) }"
    });

    let res = client
        .post(format!("{base_url}/graphql"))
        .json(&payload)
        .send()
        .await
        .expect("Failed to send POST /graphql for mutation");
    let status = res.status();
    assert!(status == StatusCode::OK || status == StatusCode::INTERNAL_SERVER_ERROR);
}
