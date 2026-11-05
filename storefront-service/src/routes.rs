use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower::ServiceBuilder;

use crate::openapi;
use crate::{handlers::checkout, handlers::health, handlers::order, handlers::payment};
use crate::{middleware::auth::AuthLayer, state::AppState};

pub fn build(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .route("/api/v1/checkout", post(checkout::checkout))
        .route("/api/v1/orders/{id}", get(order::get))
        .route("/api/v1/orders/{id}/cancel", post(order::cancel))
        .route("/api/v1/orders/{id}/pay", post(order::pay))
        .layer(ServiceBuilder::new().layer(AuthLayer {}))
        .with_state(state.clone());

    let public = Router::new()
        .route("/health", get(health::health))
        .route("/ready", get(health::ready))
        .with_state(state.clone());

    let docs = openapi::router();

    Router::new()
        .merge(public)
        .merge(protected)
        .merge(docs)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
