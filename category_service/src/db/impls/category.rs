use chrono::{DateTime, Utc};
use sqlx::PgPool;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct CategoryRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub parent_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CategoryRepo;

impl CategoryRepo {
    pub async fn get(pool: &PgPool, id: i64) -> AppResult<CategoryRow> {
        sqlx::query_as::<_, CategoryRow>("SELECT * FROM categories WHERE id = $1")
            .bind(id).fetch_optional(pool).await?
            .ok_or_else(|| AppError::NotFound(format!("category {id}")))
    }

    pub async fn list(pool: &PgPool, parent_id: Option<i64>, limit: i64, offset: i64) -> AppResult<Vec<CategoryRow>> {
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new("SELECT * FROM categories WHERE 1=1");
        if let Some(pid) = parent_id {
            qb.push(" AND parent_id = ").push_bind(pid);
        }
        qb.push(" ORDER BY id LIMIT ").push_bind(limit).push(" OFFSET ").push_bind(offset);
        Ok(qb.build_query_as::<CategoryRow>().fetch_all(pool).await?)
    }

    pub async fn insert(pool: &PgPool, name: &str, slug: &str, parent_id: Option<i64>) -> AppResult<i64> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO categories (name, slug, parent_id) VALUES ($1,$2,$3) RETURNING id",
        ).bind(name).bind(slug).bind(parent_id).fetch_one(pool).await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => AppError::Conflict("slug exists".into()),
            other => AppError::Database(other),
        })?;
        Ok(id)
    }

    pub async fn update(pool: &PgPool, id: i64, name: Option<&str>, slug: Option<&str>, parent_id: Option<Option<i64>>) -> AppResult<()> {
        let res = sqlx::query(
            r#"UPDATE categories SET
                 name = COALESCE($2, name),
                 slug = COALESCE($3, slug),
                 parent_id = COALESCE($4, parent_id),
                 updated_at = NOW()
               WHERE id = $1"#,
        ).bind(id).bind(name).bind(slug).bind(parent_id.flatten())
        .execute(pool).await?;
        if res.rows_affected() == 0 { return Err(AppError::NotFound(format!("category {id}"))); }
        Ok(())
    }

    pub async fn delete(pool: &PgPool, id: i64) -> AppResult<()> {
        let res = sqlx::query("DELETE FROM categories WHERE id = $1").bind(id).execute(pool).await?;
        if res.rows_affected() == 0 { return Err(AppError::NotFound(format!("category {id}"))); }
        Ok(())
    }
}