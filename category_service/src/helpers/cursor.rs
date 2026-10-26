use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cursor { pub id: i64 }

impl Cursor {
    pub fn encode(&self) -> String {
        general_purpose::URL_SAFE_NO_PAD.encode(self.id.to_string())
    }
    pub fn decode(s: &str) -> Option<Self> {
        let bytes = general_purpose::URL_SAFE_NO_PAD.decode(s).ok()?;
        let id = String::from_utf8(bytes).ok()?.parse().ok()?;
        Some(Self { id })
    }
}

pub fn decode_required(s: &str) -> AppResult<Cursor> {
    Cursor::decode(s).ok_or_else(|| AppError::Validation("bad cursor".into()))
}