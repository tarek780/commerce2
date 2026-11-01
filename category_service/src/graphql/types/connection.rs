use async_graphql::SimpleObject;
use crate::graphql::pagination::PageInfo;
use crate::graphql::types::product::Product;

#[derive(SimpleObject)]
pub struct ProductEdge { pub cursor: String, pub node: Product }

#[derive(SimpleObject)]
pub struct ProductConnection {
    pub edges: Vec<ProductEdge>,
    pub page_info: PageInfo,
    pub total_count: i64,
}