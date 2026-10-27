use crate::error::AppError;
use axum::{
    extract::Request,
    http::{StatusCode, header},
    response::IntoResponse,
    response::Response,
};
use futures_util::future::BoxFuture;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use std::ops::Not;
use tower::{Layer, Service};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Copy)]
pub enum Typ {
    Access,
    Refresh,
}

impl Not for Typ {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            Self::Access => Self::Refresh,
            Self::Refresh => Self::Access,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Role {
    Admin,
    User,
}

// Decode
#[derive(Debug, Serialize, Deserialize)]
pub struct DecodedToken {
    pub typ: Typ,
    pub user_id: String,
    pub token_id: String,
    pub role: Role,
    pub exp: i64,
}

impl DecodedToken {
    pub fn decode_jwt(token: &str) -> Result<Self, AppError> {
        let secret = std::env::var("SECRET_KEY")?;
        let validation = Validation::new(Algorithm::HS256);
        let decoded = decode::<Self>(
            token,
            &DecodingKey::from_secret(&secret.into_bytes()),
            &validation,
        )?;
        Ok(decoded.claims)
    }
}

#[derive(Clone)]
pub struct AuthLayer {}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthService { inner }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
}

impl<S> Service<Request> for AuthService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let token_opt = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));

        let token = match token_opt {
            Some(t) => t.to_string(),
            None => {
                return Box::pin(async move { Ok(StatusCode::UNAUTHORIZED.into_response()) });
            }
        };

        Box::pin(async move {
            match DecodedToken::decode_jwt(&token) {
                Ok(token_data) => {
                    let data = AccessToken {
                        user_id: token_data.user_id,
                    };
                    req.extensions_mut().insert(data);

                    inner.call(req).await
                }
                Err(_) => Ok(StatusCode::UNAUTHORIZED.into_response()),
            }
        })
    }
}

#[derive(Clone, Debug)]
pub struct AccessToken {
    pub user_id: String,
}
