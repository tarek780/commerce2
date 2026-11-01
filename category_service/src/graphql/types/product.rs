use crate::db::impls::product::ProductRow;
use crate::db::impls::variant::VariantRepo;
use crate::graphql::context::GqlContext;
use crate::graphql::query::ProductRowJson;
use crate::graphql::types::variant::Variant;
use async_graphql::{ComplexObject, Context, Result as GqlResult, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;
#[derive(Default, SimpleObject)]
#[graphql(complex)]
pub struct Product {
    pub id: Uuid,
    pub category_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub brand: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[ComplexObject]
impl Product {
    async fn variants(&self, ctx: &Context<'_>) -> GqlResult<Vec<Variant>> {
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let rows = VariantRepo::list_by_product(&gql.state.db, self.id).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

impl From<ProductRow> for Product {
    fn from(r: ProductRow) -> Self {
        Self {
            id: r.id,
            category_id: r.category_id,
            name: r.name,
            slug: r.slug,
            description: r.description,
            brand: r.brand,
            is_active: r.is_active,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

impl From<ProductRowJson> for Product {
    fn from(r: ProductRowJson) -> Self {
        Self {
            id: r.id,
            category_id: r.category_id,
            name: r.name,
            slug: r.slug,
            description: r.description,
            brand: r.brand,
            is_active: r.is_active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
