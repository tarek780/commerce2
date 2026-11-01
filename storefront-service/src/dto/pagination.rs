use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct PageParams {

    pub first: Option<i32>,

    pub after: Option<String>,
}

impl PageParams {
    pub fn limit(&self) -> i32 {
        self.first.unwrap_or(20).clamp(1, 100)
    }
    pub fn after_str(&self) -> String {
        self.after.clone().unwrap_or_default()
    }
}