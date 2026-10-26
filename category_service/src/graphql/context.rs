use std::sync::Arc;
use crate::state::AppState;

#[derive(Clone)]
pub struct GqlContext {
    pub state: Arc<AppState>,
}