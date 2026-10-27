use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch, post},
};

use crate::{
    handlers::{auth, user},
    middleware,
    state::AppState,
};

pub fn build_router(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/delete", post(auth::delete))
        .route("/api/v1/auth/verify-email", get(auth::verify_email));

    let protected = Router::new()
        .route("/api/v1/users/me", get(user::get_me))
        .route("/api/v1/users/me", patch(user::update_me))
        .layer(middleware::auth_layer::AuthLayer {});
    // AuthLayer::new(state.config.clone())

    public.merge(protected).with_state(state)
}
