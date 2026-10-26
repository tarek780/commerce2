// src/state.rs
use std::sync::Arc;
use crate::cart::service::CartService;

pub struct AppState {
    pub cart_service: Arc<CartService>,
}
