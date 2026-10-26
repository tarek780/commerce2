use crate::helpers::cursor::Cursor;
use async_graphql::{InputObject, SimpleObject};

#[derive(InputObject, Default)]
pub struct PageInput {
    pub first: Option<i32>,
    pub after: Option<String>,
}

impl PageInput {
    pub fn limit(&self) -> i64 {
        self.first.unwrap_or(20).clamp(1, 100) as i64
    }
    pub fn cursor(&self) -> Option<Cursor> {
        self.after.as_deref().and_then(|s| Cursor::decode(s))
    }
}

#[derive(SimpleObject)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

#[derive(InputObject, Default)]
pub struct ProductFilterInput {
    pub category_id: Option<i64>,
    pub brand: Option<String>,
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
}
