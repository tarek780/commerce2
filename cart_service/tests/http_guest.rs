mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use cart_service::cart::repository::GuestCartRepo;
use cart_service::cart::service::CartService;
use cart_service::state::AppState;
use serde_json::Value;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn http_guest_flow_create_add_get() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };
    let app = common::build_router(env.state.clone());
    // create
    // POST /api/guest/cart
    let res = match app
        .clone()
        .oneshot(
            match Request::builder()
                .method("POST")
                .uri("/api/guest/cart")
                .body(Body::empty())
            {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            },
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };

    assert_eq!(res.status(), StatusCode::OK);
    //
    let body = match axum::body::to_bytes(res.into_body(), usize::MAX).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    let json: Value = serde_json::from_slice(&body).unwrap();
    let guest_id = json["guest_cart_id"].as_str().unwrap().to_string();

    // POST /api/guest/cart/:id/items
    let uuid = Uuid::now_v7();
    let p = cart_service::cart::model::AddItemRequest {
        variant_id: uuid,
        quantity: 1,
        price: 100.0,
    };

    let payload = serde_json::to_string(&p).unwrap();
    // add
    let res = match app
        .clone()
        .oneshot(
            match Request::builder()
                .method("POST")
                .uri(format!("/api/guest/cart/{guest_id}/items"))
                .header("content-type", "application/json")
                .body(Body::from(payload))
            {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            },
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert_eq!(res.status(), StatusCode::OK);

    // GET /api/guest/cart/:id
    let res = match app
        .oneshot(
            match Request::builder()
                .method("GET")
                .uri(format!("/api/guest/cart/{guest_id}"))
                .body(Body::empty())
            {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            },
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert_eq!(res.status(), StatusCode::OK);
    let body = match axum::body::to_bytes(res.into_body(), usize::MAX).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    let json: Value = serde_json::from_slice(&body).unwrap();
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

    match env.state.cart_service.guest_repo.delete(&guest_id).await {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
}

#[tokio::test]
async fn http_missing_guest_cart_returns_404() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };
    let app = common::build_router(env.state.clone());

    let res = match app
        .oneshot(
            match Request::builder()
                .method("GET")
                .uri("/api/guest/cart/nonexistent-id")
                .body(Body::empty())
            {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {e}");
                    return;
                }
            },
        )
        .await
    {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: {e}");
            return;
        }
    };
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

/// all fn is running = ok but filled if you try to test them by routes because we need redis and db
#[tokio::test]
async fn c() {
    let Some(env) = common::build_state().await else {
        eprintln!("skip: services unavailable");
        return;
    };
    let state = State(env.state);

    let x = match cart_service::cart::handlers::create_guest_cart(state).await {
        Ok(o) => {
            let d = o.into_response();
            assert_eq!(d.status(), StatusCode::OK);
            "StatusCode::OK".to_string()
        }
        Err(e) => format!("{e}"),
    };
}
