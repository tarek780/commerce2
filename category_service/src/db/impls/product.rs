use chrono::{DateTime, Utc};
use sqlx::PgPool;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct ProductRow {
    pub id: i64,
    pub category_id: Option<i64>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub brand: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct ProductFilter {
    pub category_id: Option<i64>,
    pub brand: Option<String>,
    pub search: Option<String>,
    pub is_active: Option<bool>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
}

pub struct ProductRepo;

impl ProductRepo {
    pub async fn get(pool: &PgPool, id: i64) -> AppResult<ProductRow> {
        sqlx::query_as::<_, ProductRow>("SELECT * FROM products WHERE id = $1")
            .bind(id).fetch_optional(pool).await?
            .ok_or_else(|| AppError::NotFound(format!("product {id}")))
    }

    pub async fn get_by_slug(pool: &PgPool, slug: &str) -> AppResult<ProductRow> {
        sqlx::query_as::<_, ProductRow>("SELECT * FROM products WHERE slug = $1")
            .bind(slug).fetch_optional(pool).await?
            .ok_or_else(|| AppError::NotFound(format!("product {slug}")))
    }

    /// filters + cursor pagination (id-based)
    pub async fn list(
        pool: &PgPool,
        filter: &ProductFilter,
        after_id: Option<i64>,
        limit: i64,
    ) -> AppResult<(Vec<ProductRow>, bool)> {
        let fetch = limit + 1;
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new("SELECT * FROM products WHERE 1=1");
        if let Some(cid) = filter.category_id { qb.push(" AND category_id = ").push_bind(cid); }
        if let Some(b) = &filter.brand { qb.push(" AND brand = ").push_bind(b.clone()); }
        if let Some(s) = &filter.is_active { qb.push(" AND is_active = ").push_bind(*s); }
        if let Some(q) = &filter.search {
            let pattern = format!("%{q}%");
            qb.push(" AND (name ILIKE ").push_bind(pattern.clone()).push(" OR description ILIKE ").push_bind(pattern).push(")");
        }
        if let Some(aid) = after_id { qb.push(" AND id > ").push_bind(aid); }
        qb.push(" ORDER BY id ASC LIMIT ").push_bind(fetch);

        let mut rows = qb.build_query_as::<ProductRow>().fetch_all(pool).await?;
        let has_more = rows.len() as i64 > limit;
        if has_more { rows.truncate(limit as usize); }
        Ok((rows, has_more))
    }

    pub async fn insert(
        pool: &PgPool,
        category_id: Option<i64>, name: &str, slug: &str,
        description: Option<&str>, brand: Option<&str>, is_active: bool,
    ) -> AppResult<i64> {
        let id: i64 = sqlx::query_scalar(
            r#"INSERT INTO products (category_id, name, slug, description, brand, is_active)
               VALUES ($1,$2,$3,$4,$5,$6) RETURNING id"#,
        ).bind(category_id).bind(name).bind(slug).bind(description).bind(brand).bind(is_active)
        .fetch_one(pool).await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => AppError::Conflict("slug exists".into()),
            other => AppError::Database(other),
        })?;
        Ok(id)
    }

    pub async fn update(
        pool: &PgPool, id: i64,
        category_id: Option<Option<i64>>, name: Option<&str>, slug: Option<&str>,
        description: Option<&str>, brand: Option<&str>, is_active: Option<bool>,
    ) -> AppResult<()> {
        let res = sqlx::query(
            r#"UPDATE products SET
                 category_id = COALESCE($2, category_id),
                 name = COALESCE($3, name),
                 slug = COALESCE($4, slug),
                 description = COALESCE($5, description),
                 brand = COALESCE($6, brand),
                 is_active = COALESCE($7, is_active),
                 updated_at = NOW()
               WHERE id = $1"#,
        ).bind(id).bind(category_id.flatten()).bind(name).bind(slug)
         .bind(description).bind(brand).bind(is_active)
         .execute(pool).await?;
        if res.rows_affected() == 0 { return Err(AppError::NotFound(format!("product {id}"))); }
        Ok(())
    }

    pub async fn delete(pool: &PgPool, id: i64) -> AppResult<()> {
        let res = sqlx::query("DELETE FROM products WHERE id = $1").bind(id).execute(pool).await?;
        if res.rows_affected() == 0 { return Err(AppError::NotFound(format!("product {id}"))); }
        Ok(())
    }
}