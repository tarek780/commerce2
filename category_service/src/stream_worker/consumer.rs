use redis::aio::ConnectionManager;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::AsyncCommands;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::db::impls::category::CategoryRepo;
use crate::db::impls::product::ProductRepo;
use crate::db::impls::variant::VariantRepo;
use crate::redis::{json, stream::CatalogEvent};
use crate::state::AppState;

pub fn spawn(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut redis = state.redis.clone();
        if let Err(e) = ensure_group(
            &mut redis,
            &state.cfg.redis_stream_key,
            &state.cfg.redis_consumer_group,
        )
        .await
        {
            error!(?e, "ensure_group failed");
            return;
        }
        info!("catalog stream consumer started");
        loop {
            match read_batch(
                &mut redis,
                &state.cfg.redis_stream_key,
                &state.cfg.redis_consumer_group,
                &state.cfg.redis_consumer_name,
                5000,
                16,
            )
            .await
            {
                Ok(reply) => {
                    for (id, fields) in parse_reply(reply) {
                        if let Err(e) = handle_event(&state, &mut redis, &fields).await {
                            error!(?e, %id, "event processing failed");
                            continue; // لا ACK → retry
                        }
                        let _: i64 = redis::cmd("XACK")
                            .arg(&state.cfg.redis_stream_key)
                            .arg(&state.cfg.redis_consumer_group)
                            .arg(&id)
                            .query_async(&mut redis)
                            .await
                            .unwrap_or(0);
                    }
                }
                Err(e) => {
                    warn!(?e, "read_batch failed");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });
}

async fn ensure_group(
    mgr: &mut ConnectionManager,
    stream: &str,
    group: &str,
) -> redis::RedisResult<()> {
    let res: Result<String, redis::RedisError> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(stream)
        .arg(group)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(mgr)
        .await;
    match res {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("BUSYGROUP") => Ok(()),
        Err(e) => Err(e),
    }
}

async fn read_batch(
    mgr: &mut ConnectionManager,
    stream: &str,
    group: &str,
    consumer: &str,
    block_ms: usize,
    count: usize,
) -> redis::RedisResult<StreamReadReply> {
    let opts = StreamReadOptions::default()
        .group(group, consumer)
        .block(block_ms)
        .count(count)
        .noack();
    mgr.xread_options(&[stream], &[">"], &opts).await
}

fn parse_reply(reply: StreamReadReply) -> Vec<(String, HashMap<String, redis::Value>)> {
    let mut out = Vec::new();
    for key in reply.keys {
        for entry in key.ids {
            let mut map = HashMap::new();
            for (k, v) in entry.map {
                map.insert(k, v);
            }
            out.push((entry.id, map));
        }
    }
    out
}

async fn handle_event(
    state: &Arc<AppState>,
    redis: &mut ConnectionManager,
    fields: &HashMap<String, redis::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let kind = value_to_string(fields.get("kind"));
    let id_str = value_to_string(fields.get("id"));
    let payload_str = value_to_string(fields.get("payload"));
    let payload: Value = payload_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);

    let event: CatalogEvent = serde_json::from_value(serde_json::json!({
        "kind": kind,
        "id": id_str.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0),
        "payload": payload,
    }))?;

    match event {
        CatalogEvent::CategoryCreated { payload, .. } => {
            let name = payload["name"].as_str().unwrap_or("");
            let slug = payload["slug"].as_str().unwrap_or("");
            let pid = payload["parent_id"].as_i64();
            let id = CategoryRepo::insert(&state.db, name, slug, pid).await?;
            let row = CategoryRepo::get(&state.db, id).await?;
            let _ = json::set_json(
                redis,
                &format!("category:{id}"),
                &serde_json::to_value(&row)?,
            )
            .await;
        }
        CatalogEvent::ProductCreated { payload, .. } => {
            let name = payload["name"].as_str().unwrap_or("");
            let slug = payload["slug"].as_str().unwrap_or("");
            let id = ProductRepo::insert(
                &state.db,
                payload["category_id"].as_i64(),
                name,
                slug,
                payload["description"].as_str(),
                payload["brand"].as_str(),
                payload["is_active"].as_bool().unwrap_or(true),
            )
            .await?;
            let row = ProductRepo::get(&state.db, id).await?;
            let _ = json::set_json(
                redis,
                &format!("product:{id}"),
                &serde_json::to_value(&row)?,
            )
            .await;
        }
        CatalogEvent::ProductUpdated { id, payload } => {
            ProductRepo::update(
                &state.db,
                id,
                payload["category_id"].as_i64().map(Some),
                payload["name"].as_str(),
                payload["slug"].as_str(),
                payload["description"].as_str(),
                payload["brand"].as_str(),
                payload["is_active"].as_bool(),
            )
            .await?;
            let row = ProductRepo::get(&state.db, id).await?;
            let _ = json::set_json(
                redis,
                &format!("product:{id}"),
                &serde_json::to_value(&row)?,
            )
            .await;

            // variant marker → create variant
            if payload.get("sku").is_some() {
                let sku = payload["sku"].as_str().unwrap_or("");
                let price: sqlx::types::BigDecimal =
                    payload["price"].as_str().unwrap_or("0").parse()?;
                let attrs = payload.get("attributes").cloned().unwrap_or(Value::Null);
                let vid = VariantRepo::insert(
                    &state.db,
                    id,
                    sku,
                    price,
                    None,
                    payload["image_url"].as_str(),
                    &attrs,
                    payload["is_active"].as_bool().unwrap_or(true),
                )
                .await?;

                // gRPC → inventory
                let _ = state.inventory.ensure_sku(vid as i32).await;
            }
        }
        CatalogEvent::ProductDeleted { id, .. } => {
            ProductRepo::delete(&state.db, id).await?;
            let _ = json::del_json(redis, &format!("product:{id}")).await;
        }
        CatalogEvent::CategoryUpdated { id, payload } => {
            CategoryRepo::update(
                &state.db,
                id,
                payload["name"].as_str(),
                payload["slug"].as_str(),
                payload["parent_id"].as_i64().map(Some),
            )
            .await?;
            let row = CategoryRepo::get(&state.db, id).await?;
            let _ = json::set_json(
                redis,
                &format!("category:{id}"),
                &serde_json::to_value(&row)?,
            )
            .await;
        }
        CatalogEvent::CategoryDeleted { id, .. } => {
            CategoryRepo::delete(&state.db, id).await?;
            let _ = json::del_json(redis, &format!("category:{id}")).await;
        }
    }
    Ok(())
}

fn value_to_string(v: Option<&redis::Value>) -> Option<String> {
    use redis::Value as V;

    match v? {
        V::Data(b) => Some(String::from_utf8_lossy(b).into_owned()),
        V::Status(s) => Some(s.clone()),
        V::Int(i) => Some(i.to_string()),
        _ => None,
    }
}
