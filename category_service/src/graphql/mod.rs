pub mod context;
pub mod mutation;
pub mod pagination;
pub mod query;

pub mod schema;
pub mod types;

use crate::state::AppState;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::response::Html;
use axum::{routing::get, Router};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn router(state: Arc<AppState>) -> Router {
    let schema = schema::build();

    Router::new()
        .route("/graphql", get(playground).post(handler))
        .with_state((state, schema))
        .layer(TraceLayer::new_for_http())
}

async fn playground() -> Html<String> {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

async fn handler(
    axum::extract::State((state, schema)): axum::extract::State<(Arc<AppState>, schema::AppSchema)>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let ctx = context::GqlContext { state };
    schema.execute(req.into_inner().data(ctx)).await.into()
}
