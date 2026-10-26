

use chrono::Utc;
use redis::AsyncCommands;
use sqlx::PgPool;
use uuid::Uuid;

use crate::cart::model::{AddItemRequest, CachedCart, CartItem};
use crate::error::CartError;

const GUEST_TTL_SECS: u64 = 7 * 24 * 3600;
const USER_CACHE_TTL_SECS: u64 = 3600;

fn guest_key(id: &str) -> String {
    format!("cart:guest:{id}")
}
fn user_key(id: &str) -> String {
    format!("cart:user:{id}")
}

//  Guest = Redis only
pub struct GuestCartRepo {
    pub redis: redis::aio::MultiplexedConnection,
}

impl GuestCartRepo {
    pub async fn create(&self) -> Result<String, CartError> {
        let guest_id = Uuid::now_v7().to_string();
        let payload = CachedCart {
            items: vec![],
            updated_at: Utc::now(),
        };
        self.put(&guest_id, &payload).await?;
        Ok(guest_id)
    }

    pub async fn get(&self, guest_id: &str) -> Result<CachedCart, CartError> {
        let mut conn = self.redis.clone();
        let raw: Option<String> = redis::cmd("GET")
            .arg(guest_key(guest_id))
            .query_async(&mut conn)
            .await?;
        match raw {
            Some(s) => Ok(serde_json::from_str(&s)?),
            None => Err(CartError::NotFound),
        }
    }

    pub async fn put(&self, guest_id: &str, cart: &CachedCart) -> Result<(), CartError> {
        let json = serde_json::to_string(cart)?;
        let mut conn = self.redis.clone();
        let _: () = redis::cmd("SET")
            .arg(guest_key(guest_id))
            .arg(json)
            .arg("EX")
            .arg(GUEST_TTL_SECS)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn delete(&self, guest_id: &str) -> Result<(), CartError> {
        let mut conn = self.redis.clone();

        let _: () = conn.del(guest_key(guest_id)).await?;
        Ok(())
    }
}

//  User := PG + Redis 
pub struct UserCartRepo {
    pub pool: PgPool,
    pub redis: redis::aio::MultiplexedConnection,
}

impl UserCartRepo {
    pub async fn get_or_create_cart_id(&self, user_id: &str) -> Result<String, CartError> {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT id FROM carts WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await?;

        if let Some(id) = existing {
            return Ok(id);
        }

        let cart_id = Uuid::now_v7().to_string();
        sqlx::query("INSERT INTO carts (id, user_id) VALUES ($1, $2)")
            .bind(&cart_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(cart_id)
    }


    pub async fn upsert_items_bulk(
        &self,
        cart_id: &str,
        items: &[AddItemRequest],
    ) -> Result<(), CartError> {
        if items.is_empty() {
            return Ok(());
        }

        let payload: Vec<serde_json::Value> = items
            .iter()
            .map(|i| {
                serde_json::json!({
                    "id":       Uuid::now_v7().to_string(),
                    "sku_id":   i.sku_id,
                    "quantity": i.quantity,
                    "price":    i.price,
                })
            })
            .collect();

        let payload_json = serde_json::Value::Array(payload);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO cart_items (id, cart_id, sku_id, quantity, price_at_add)
            SELECT
                (elem->>'id')::text,
                $1,
                (elem->>'sku_id')::text,
                (elem->>'quantity')::int,
                (elem->>'price')::float8
            FROM jsonb_array_elements($2::jsonb) AS elem
            ON CONFLICT (cart_id, sku_id)
            DO UPDATE SET
                quantity     = cart_items.quantity + EXCLUDED.quantity,
                price_at_add = EXCLUDED.price_at_add
            "#,
        )
        .bind(cart_id)
        .bind(&payload_json)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE carts SET updated_at = now() WHERE id = $1")
            .bind(cart_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn load_items(&self, cart_id: &str) -> Result<Vec<CartItem>, CartError> {
        let rows: Vec<(String, i32, f64, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            "SELECT sku_id, quantity, price_at_add::float8, added_at \
             FROM cart_items WHERE cart_id = $1 ORDER BY added_at",
        )
        .bind(cart_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(sku_id, quantity, price_at_add, added_at)| CartItem {
                sku_id,
                quantity,
                price_at_add,
                added_at,
            })
            .collect())
    }

    // ---- Redis cache for user cart ----
    pub async fn get_cached(&self, user_id: &str) -> Result<Option<CachedCart>, CartError> {
        let mut conn = self.redis.clone();
        let raw: Option<String> = conn.get(user_key(user_id)).await?;
        match raw {
            Some(s) => Ok(Some(serde_json::from_str(&s)?)),
            None => Ok(None),
        }
    }

    pub async fn put_cached(&self, user_id: &str, cart: &CachedCart) -> Result<(), CartError> {
        let json = serde_json::to_string(cart)?;
        let mut conn = self.redis.clone();
        let _: () = conn
            .set_ex(user_key(user_id), json, USER_CACHE_TTL_SECS)
            .await?;
        Ok(())
    }

    pub async fn invalidate_cache(&self, user_id: &str) -> Result<(), CartError> {
        let mut conn = self.redis.clone();
        let _: () = conn.del(user_key(user_id)).await?;
        Ok(())
    }
}
