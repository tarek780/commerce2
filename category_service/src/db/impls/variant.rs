use crate::error::{AppError, AppResult};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct VariantRow {
    pub id: Uuid,
    pub product_id: Uuid,
    pub sku: String,
    pub price: BigDecimal,
    pub compare_at_price: Option<BigDecimal>,
    pub image_url: Option<String>,
    pub attributes: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
/// error communicating with database: Connection refused (os error 111)

pub struct VariantRepo;

impl VariantRepo {
    pub async fn list_by_product(pool: &PgPool, product_id: Uuid) -> AppResult<Vec<VariantRow>> {
        Ok(sqlx::query_as::<_, VariantRow>(
            "SELECT * FROM product_variants WHERE product_id = $1 ORDER BY id",
        )
        .bind(product_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn insert(
        pool: &PgPool,
        product_id: &str,
        sku: &str,
        price: BigDecimal,
        compare_at_price: Option<BigDecimal>,
        image_url: Option<&str>,
        attributes: &serde_json::Value,
        is_active: bool,
    ) -> AppResult<String> {
        let product_id = Uuid::parse_str(product_id)?;
        let id = sqlx::query_scalar(
            r#"INSERT INTO product_variants
               (product_id, sku, price, compare_at_price, image_url, attributes, is_active)
               VALUES ($1,$2,$3,$4,$5,$6,$7 , $8) RETURNING id"#,
        )
        .bind(Uuid::now_v7())
        .bind(product_id)
        .bind(sku)
        .bind(price)
        .bind(compare_at_price)
        .bind(image_url)
        .bind(attributes)
        .bind(is_active)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(d) if d.is_unique_violation() => {
                AppError::Conflict("sku exists".into())
            }
            other => AppError::Database(other),
        })?;
        Ok(id)
    }

    pub async fn delete(pool: &PgPool, id: Uuid) -> AppResult<()> {
        let res = sqlx::query("DELETE FROM product_variants WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("variant {id}")));
        }
        Ok(())
    }
}
