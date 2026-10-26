use async_graphql::{EmptySubscription, Schema};
use crate::graphql::mutation::MutationRoot;
use crate::graphql::query::QueryRoot;

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build() -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription).finish()
}