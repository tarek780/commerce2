use crate::db::impls::category::CategoryRepo;
use crate::db::impls::product::{ProductFilter, ProductRepo};
use crate::graphql::context::GqlContext;
use crate::graphql::pagination::{PageInfo, PageInput, ProductFilterInput};
use crate::graphql::types::category::Category;
use crate::graphql::types::connection::{ProductConnection, ProductEdge};
use crate::graphql::types::product::Product;
use crate::helpers::cursor::Cursor;
use crate::redis::{json, search};
use async_graphql::{Context, Object, Result as GqlResult, ID};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn products(
        &self,
        ctx: &Context<'_>,
        filter: Option<ProductFilterInput>,
        page: Option<PageInput>,
    ) -> GqlResult<ProductConnection> {
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let page = page.unwrap_or_default();
        let filter = filter.unwrap_or_default();
        let limit = page.limit() as usize;

        let q = build_redis_query(&filter);
        let mut docs = search::search_products(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.index_products,
            &q,
            0,
            limit,
        )
        .await
        .unwrap_or_default();

        if docs.len() < limit {
            let missing = (limit - docs.len()) as i64;
            let repo_filter = ProductFilter {
                category_id: filter.category_id,
                brand: filter.brand.clone(),
                search: filter.search.clone(),
                is_active: Some(filter.is_active.unwrap_or(true)),
                min_price: filter.min_price,
                max_price: filter.max_price,
            };
            let (rows, _) = ProductRepo::list(
                &gql.state.db,
                &repo_filter,
                page.cursor().map(|c| c.id),
                missing,
            )
            .await?;
            for r in &rows {
                let doc = product_to_json(r);
                let _ = json::set_json(
                    &mut gql.state.redis.clone(),
                    &format!("product:{}", r.id),
                    &doc,
                )
                .await;
                docs.push(doc);
            }
        }

        let mut edges = Vec::with_capacity(docs.len());
        for d in docs {
            let id: i64 = d.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let cursor = Cursor { id }.encode();
            let node: Product = serde_json::from_value::<ProductRowJson>(d.clone())
                .map(Product::from)
                .unwrap_or_else(|_| Product::from(ProductRowFallback::default_row()));
            edges.push(ProductEdge { cursor, node });
        }

        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());
        let total_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM products")
            .fetch_one(&gql.state.db)
            .await
            .unwrap_or(0);

        Ok(ProductConnection {
            edges,
            page_info: PageInfo {
                has_next_page: false,
                has_previous_page: page.after.is_some(),
                start_cursor,
                end_cursor,
            },
            total_count,
        })
    }

    async fn category(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Category> {
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let iid: i64 = id.parse()?;
        let key = format!("category:{iid}");
        if let Some(v) = json::get_json(&mut gql.state.redis.clone(), &key).await? {
            if let Ok(row) = serde_json::from_value::<crate::db::impls::category::CategoryRow>(v) {
                return Ok(row.into());
            }
        }
        let row = CategoryRepo::get(&gql.state.db, iid).await?;
        let _ = json::set_json(
            &mut gql.state.redis.clone(),
            &key,
            &serde_json::to_value(&row)?,
        )
        .await;
        Ok(row.into())
    }

    async fn product(&self, ctx: &Context<'_>, id: ID) -> GqlResult<Product> {
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        let iid: i64 = id.parse()?;
        let key = format!("product:{iid}");
        if let Some(v) = json::get_json(&mut gql.state.redis.clone(), &key).await? {
            if let Ok(row) = serde_json::from_value::<crate::db::impls::product::ProductRow>(v) {
                return Ok(row.into());
            }
        }
        let row = ProductRepo::get(&gql.state.db, iid).await?;
        let _ = json::set_json(
            &mut gql.state.redis.clone(),
            &key,
            &serde_json::to_value(&row)?,
        )
        .await;
        Ok(row.into())
    }

    async fn categories(
        &self,
        ctx: &Context<'_>,
        parent_id: Option<i64>,
    ) -> GqlResult<Vec<Category>> {
        let gql = ctx.data_unchecked::<GqlContext>().clone();
        // search بسيط
        let q = match parent_id {
            Some(p) => format!("@parent_id:[{p} {p}]"),
            None => "*".into(),
        };
        let docs = search::search_products(
            &mut gql.state.redis.clone(),
            &gql.state.cfg.index_categories,
            &q,
            0,
            100,
        )
        .await
        .unwrap_or_default();

        if docs.is_empty() {
            let rows = CategoryRepo::list(&gql.state.db, parent_id, 100, 0).await?;
            let mut out = Vec::new();
            for r in rows {
                let _ = json::set_json(
                    &mut gql.state.redis.clone(),
                    &format!("category:{}", r.id),
                    &serde_json::to_value(&r)?,
                )
                .await;
                out.push(r.into());
            }
            return Ok(out);
        }

        Ok(docs
            .into_iter()
            .filter_map(|d| {
                serde_json::from_value::<crate::db::impls::category::CategoryRow>(d)
                    .ok()
                    .map(Into::into)
            })
            .collect())
    }
}

//  Helpers

fn build_redis_query(f: &ProductFilterInput) -> String {
    let mut parts = vec!["*".to_string()];
    //
    if let Some(cid) = f.category_id {
        parts.push(format!("@category_id:[{cid} {cid}]"));
    }
    if let Some(b) = &f.brand {
        parts.push(format!("@brand:{{{b}}}"));
    }
    if let Some(s) = &f.search {
        parts.push(format!("@name:{s}*"));
    }
    if let Some(act) = f.is_active {
        parts.push(format!(
            "@is_active:{{{}}}",
            if act { "true" } else { "false" }
        ));
    }
    if let Some(min) = f.min_price {
        parts.push(format!("@price_min:[{min} +inf]"));
    }
    if let Some(max) = f.max_price {
        parts.push(format!("@price_min:[-inf {max}]"));
    }
    parts.join(" ")
}

fn product_to_json(r: &crate::db::impls::product::ProductRow) -> serde_json::Value {
    serde_json::json!({
        "id": r.id,
        "category_id": r.category_id,
        "name": r.name,
        "slug": r.slug,
        "description": r.description,
        "brand": r.brand,
        "is_active": r.is_active,
    })
}

//wrappers
#[derive(serde::Deserialize, Default)]
pub(crate) struct ProductRowJson {
    pub(crate) id: i64,
    pub(crate) category_id: Option<i64>,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) description: Option<String>,
    pub(crate) brand: Option<String>,
    pub(crate) is_active: bool,
}
impl From<ProductRowJson> for crate::db::impls::product::ProductRow {
    fn from(j: ProductRowJson) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: j.id,
            category_id: j.category_id,
            name: j.name,
            slug: j.slug,
            description: j.description,
            brand: j.brand,
            is_active: j.is_active,
            created_at: now,
            updated_at: now,
        }
    }
}

struct ProductRowFallback;
impl ProductRowFallback {
    fn default_row() -> crate::db::impls::product::ProductRow {
        let now = chrono::Utc::now();
        crate::db::impls::product::ProductRow {
            id: 0,
            category_id: None,
            name: "".into(),
            slug: "".into(),
            description: None,
            brand: None,
            is_active: false,
            created_at: now,
            updated_at: now,
        }
    }
}
