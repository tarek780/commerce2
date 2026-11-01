
use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use tower::ServiceBuilder;

use crate::{state::AppState, middleware::auth::AuthLayer};
use crate::{handlers::health, handlers::orders, handlers::checkout, handlers::payments};

pub fn build(state: Arc<AppState>) -> Router {

    let protected = Router::new()
        .route("/api/v1/checkout", post(checkout::checkout))
        .route("/api/v1/orders", get(orders::list_orders))
        .route("/api/v1/orders/{id}", get(orders::get_order))
        .route("/api/v1/orders/{id}/cancel", post(orders::cancel_order))
        .route("/api/v1/orders/{id}/pay", post(orders::pay_order))
        .route("/api/v1/payments/{id}", get(payments::get_payment))
        .layer(ServiceBuilder::new().layer(AuthLayer {}))
        .with_state(state.clone());


    let public = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(state.clone());

    Router::new()
        .merge(public)
        .merge(protected)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}