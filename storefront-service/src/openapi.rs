use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::dto::{
    checkout::CheckoutResponse,
    order::{CancelOrderBody, OrderConnection, OrderDto, OrderEdge, PageInfoDto, PayOrderResponse},
    pagination::PageParams,
    payment::PaymentDto,
};
// use crate::handlers::{
//     checkout::*,
//     health::{health, ready},
//     order::{cancel, get, pay},
//     payment::*,
// };

use crate::problem::Problem;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Storefront API",
        version = "1.0.0",
        description = "BFF  Order/Payment gRPC  cart/category REST."
    ),
    paths(
        crate::handlers::checkout::checkout,
        crate::handlers::order::get,
        crate::handlers::order::cancel,
        crate::handlers::order::pay,
        crate::handlers::payment::get,
        crate::handlers::health::health,
        crate::handlers::health::ready,
    ),
    components(
        schemas(
            CheckoutResponse,
            OrderDto, OrderEdge, OrderConnection, PageInfoDto, PayOrderResponse, CancelOrderBody,
            PaymentDto, PageParams, Problem,
        )
    ),
    tags(
        (name = "storefront", description = "Storefront BFF")
    )
)]
pub struct ApiDoc;

pub fn router() -> Router {
    Router::new().merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
}
