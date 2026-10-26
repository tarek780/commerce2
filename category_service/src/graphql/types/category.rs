use async_graphql::{ComplexObject, Context, Result as GqlResult, SimpleObject, ID};
use chrono::{DateTime, Utc};
use crate::graphql::context::GqlContext;
use crate::db::impls::category::{CategoryRepo, CategoryRow};

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Category {
    pub id: ID,
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
        let id: i64 = self.id.parse()?;
        let rows = CategoryRepo::list(&gql.state.db, Some(id), 100, 0).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

impl From<CategoryRow> for Category {
    fn from(r: CategoryRow) -> Self {
        Self {
            id: ID(r.id.to_string()),
            name: r.name,
            slug: r.slug,
            parent_id: r.parent_id.map(|p| ID(p.to_string())),
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}