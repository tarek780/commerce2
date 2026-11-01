use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    async_trait,
};
use uuid::Uuid;
use crate::error::AppError;


#[derive(Debug, Clone)]
pub struct CurrentUser(pub Uuid);

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("x-user-id")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("missing x-user-id header".into()))?;

        let id = Uuid::parse_str(header)
            .map_err(|_| AppError::Unauthorized("invalid x-user-id".into()))?;

        Ok(CurrentUser(id))
    }
}