use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::dto::{checkout::*, order::*, pagination::*, payment::*};
use crate::problem::Problem;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Storefront API",
        version = "1.0.0",
        description = "BFF للعميل — يغلّف Order/Payment gRPC ويستخدم cart/category REST."
    ),
    paths(
        crate::handlers::checkout::checkout,
        crate::handlers::orders::list,
        crate::handlers::orders::get,
        crate::handlers::orders::cancel,
        crate::handlers::orders::pay,
        crate::handlers::payments::get,
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
    Router::new()
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
}