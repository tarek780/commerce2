use async_graphql::{Context, InputObject, Object, Result as GqlResult, ID};
use crate::graphql::context::GqlContext;
use crate::redis::stream::{publish, CatalogEvent};
use crate::middleware::auth::{Role , AccessToken};
#[derive(InputObject)]
pub struct CreateCategoryInput {
    pub name: String,
    pub slug: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(InputObject)]
pub struct CreateProductInput {
    pub category_id: Option<i64>,
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub brand: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(InputObject)]
pub struct CreateVariantInput {
    pub product_id: i64,
    pub sku: String,
    pub price: String,
    pub compare_at_price: Option<String>,
    pub image_url: Option<String>,
    pub attributes: serde_json::Value,
    pub is_active: Option<bool>,
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn create_category(
        &self,
        ctx: &Context<'_>,
        input: CreateCategoryInput,
    ) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
      
        let slug = input.slug.unwrap_or_else(|| slug::slugify(&input.name));
        let payload = serde_json::json!({
            "name": input.name, "slug": slug, "parent_id": input.parent_id
        });
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::CategoryCreated { id: 0, payload },
        ).await?;
        Ok(true)
    }

    async fn update_category(
        &self, ctx: &Context<'_>, id: ID,
        name: Option<String>, slug: Option<String>, parent_id: Option<i64>,
    ) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let iid: i64 = id.parse()?;
        let payload = serde_json::json!({
            "name": name, "slug": slug, "parent_id": parent_id
        });
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::CategoryUpdated { id: iid, payload },
        ).await?;
        Ok(true)
    }

    async fn delete_category(&self, ctx: &Context<'_>, id: ID) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let iid: i64 = id.parse()?;
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::CategoryDeleted { id: iid },
        ).await?;
        Ok(true)
    }

    async fn create_product(
        &self, ctx: &Context<'_>, input: CreateProductInput,
    ) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let slug = input.slug.unwrap_or_else(|| slug::slugify(&input.name));
        let payload = serde_json::json!({
            "category_id": input.category_id,
            "name": input.name, "slug": slug,
            "description": input.description,
            "brand": input.brand,
            "is_active": input.is_active.unwrap_or(true),
        });
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::ProductCreated { id: 0, payload },
        ).await?;
        Ok(true)
    }

    async fn update_product(
        &self, ctx: &Context<'_>, id: ID,
        category_id: Option<i64>, name: Option<String>, slug: Option<String>,
        description: Option<String>, brand: Option<String>, is_active: Option<bool>,
    ) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let iid: i64 = id.parse()?;
        let payload = serde_json::json!({
            "category_id": category_id, "name": name, "slug": slug,
            "description": description, "brand": brand, "is_active": is_active,
        });
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::ProductUpdated { id: iid, payload },
        ).await?;
        Ok(true)
    }

    async fn delete_product(&self, ctx: &Context<'_>, id: ID) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let iid: i64 = id.parse()?;
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::ProductDeleted { id: iid },
        ).await?;
        Ok(true)
    }

    async fn create_variant(
        &self, ctx: &Context<'_>, input: CreateVariantInput,
    ) -> GqlResult<bool> {
        let  role = ctx.data_unchecked::<AccessToken>().role.clone();
       if role != Role::Admin {
            return Err(async_graphql::Error::new("Unauthorized"));
        }
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let payload = serde_json::json!({
            "product_id": input.product_id,
            "sku": input.sku,
            "price": input.price,
            "compare_at_price": input.compare_at_price,
            "image_url": input.image_url,
            "attributes": input.attributes,
            "is_active": input.is_active.unwrap_or(true),
        });
       
        publish(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.redis_stream_key,
            &CatalogEvent::ProductUpdated { id: input.product_id, payload },
        ).await?;
        Ok(true)
    }
}