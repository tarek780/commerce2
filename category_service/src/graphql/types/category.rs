use crate::db::impls::category::{CategoryRepo, CategoryRow};
use crate::graphql::context::GqlContext;
use async_graphql::{ComplexObject, Context, ID, Result as GqlResult, SimpleObject};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Default, SimpleObject)]
#[graphql(complex)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<ID>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[ComplexObject]
impl Category {
    async fn children(&self, ctx: &Context<'_>) -> GqlResult<Vec<Category>> {
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let rows = CategoryRepo::list(&gql.state.db, Some(self.id), 100, 0).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

impl From<CategoryRow> for Category {
    fn from(r: CategoryRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            slug: r.slug,
            parent_id: r.parent_id.map(|p| ID(p.to_string())),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}
