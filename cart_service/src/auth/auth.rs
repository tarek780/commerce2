use crate::error::CartError;
use axum::{
    extract::Request,
    http::{StatusCode, header},
    response::IntoResponse,
    response::Response,
};
use futures_util::future::BoxFuture;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use tower::{Layer, Service};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,
    pub typ: String,
    pub role: String,
}

impl Claims {
    pub fn decode_jwt(token: &str) -> Result<Self, CartError> {
        let secret = std::env::var("SECRET_KEY")?;

        let mut validation = Validation::new(Algorithm::HS256);

        validation.leeway = 0;

        validation.validate_aud = false;
        validation.set_required_spec_claims(&["exp", "sub"]);

        let decoded = decode::<Self>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map_err(|e| CartError::Unauthorized(format!("jwt decode error: {e}")))?;

        Ok(decoded.claims)
    }
}

#[derive(Clone)]
pub struct AuthLayer {}

impl AuthLayer {
    pub fn new() -> Self {
        Self {}
    }
}

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
            match Claims::decode_jwt(&token) {
                Ok(claims) => {
                    let data = AccessToken {
                        user_id: claims.sub,
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
