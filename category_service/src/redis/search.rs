use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use redis::aio::ConnectionManager;
use redis::Value as V;
use serde_json::Value;
pub async fn ensure_indexes(mgr: &mut ConnectionManager, cfg: &AppConfig) -> AppResult<()> {
    // products index
    let _ = redis::cmd("FT.CREATE")
        .arg(&cfg.index_products)
        .arg("ON")
        .arg("JSON")
        .arg("PREFIX")
        .arg("1")
        .arg("product:")
        .arg("SCHEMA")
        .arg("$.name")
        .arg("AS")
        .arg("name")
        .arg("TEXT")
        .arg("$.brand")
        .arg("AS")
        .arg("brand")
        .arg("TAG")
        .arg("$.category_id")
        .arg("AS")
        .arg("category_id")
        .arg("NUMERIC")
        .arg("$.is_active")
        .arg("AS")
        .arg("is_active")
        .arg("TAG")
        .arg("$.price_min")
        .arg("AS")
        .arg("price_min")
        .arg("NUMERIC")
        .query_async(mgr)
        .await
        .or_else(|e| {
            if e.to_string().contains("Index already exists") {
                Ok(())
            } else {
                Err(AppError::Redis(e))
            }
        })?;

    // categories index
    let _ = redis::cmd("FT.CREATE")
        .arg(&cfg.index_categories)
        .arg("ON")
        .arg("JSON")
        .arg("PREFIX")
        .arg("1")
        .arg("category:")
        .arg("SCHEMA")
        .arg("$.name")
        .arg("AS")
        .arg("name")
        .arg("TEXT")
        .arg("$.slug")
        .arg("AS")
        .arg("slug")
        .arg("TAG")
        .arg("$.parent_id")
        .arg("AS")
        .arg("parent_id")
        .arg("NUMERIC")
        .query_async(mgr)
        .await
        .or_else(|e| {
            if e.to_string().contains("Index already exists") {
                Ok(())
            } else {
                Err(AppError::Redis(e))
            }
        })?;

    Ok(())
}

/// FT.SEARCH → Vec<serde_json::Value>
pub async fn search_products(
    mgr: &mut ConnectionManager,
    index: &str,
    query: &str,
    offset: usize,
    limit: usize,
) -> AppResult<Vec<Value>> {
    let raw: redis::Value = redis::cmd("FT.SEARCH")
        .arg(index)
        .arg(query)
        .arg("LIMIT")
        .arg(offset)
        .arg(limit)
        .arg("RETURN")
        .arg("1")
        .arg("$")
        .query_async(mgr)
        .await?;

    parse_ft_search(raw)
}

fn parse_ft_search(value: V) -> AppResult<Vec<Value>> {
    let arr = match value {
        V::Bulk(arr) => arr,
        _ => {
            return Err(AppError::Internal("Invalid FT.SEARCH reply".into()));
        }
    };

    let mut docs = Vec::new();
    let mut i = 1;

    while i + 1 < arr.len() {
        let fields = match &arr[i + 1] {
            V::Bulk(fields) => fields,
            _ => {
                i += 2;
                continue;
            }
        };

        let mut j = 0;

        while j + 1 < fields.len() {
            if let (V::Data(key), V::Data(value)) = (&fields[j], &fields[j + 1]) {
                if key.as_slice() == b"$" {
                    match serde_json::from_slice::<Value>(value) {
                        Ok(json) => {
                            if let Some(first) = json.as_array().and_then(|a| a.first()) {
                                docs.push(first.clone());
                            } else {
                                docs.push(json);
                            }
                        }
                        Err(err) => {
                            return Err(AppError::Internal(format!(
                                "Invalid JSON in FT.SEARCH result: {err}"
                            )));
                        }
                    }
                }
            }

            j += 2;
        }

        i += 2;
    }

    Ok(docs)
}

//

// pub async fn search_product(
//     mgr: &mut ConnectionManager,
//     index: &str,
//     query: &str,
//     offset: usize,
//     limit: usize,
// ) -> AppResult<JsonFromRedis> {
//     let (count, result): (i64, Vec<(String, Vec<String>)>) = redis::cmd("FT.SEARCH")
//         .arg(index)
//         .arg(query)
//         .arg("LIMIT")
//         .arg(offset)
//         .arg(limit)
//         .arg("RETURN")
//         .arg("1")
//         .arg("$")
//         .query_async(mgr)
//         .await?;

//     // for JsonFromRedis
//     let mut json_vec = Vec::new();
//     // for JsonFromRedis
//     let json_couont = json_vec.len();

//     for (_, data) in result {
//         if let json = data.len() >= 2 {
//             let json_raw = data[1].as_str();
//             let json = serde_json::to_string(json_raw)?;
//             json_vec.push(json);
//         }
//     }

//     let data = JsonFromRedis {
//         all_count: count,
//         json: json_vec,
//         json_couont: json_couont as i64,
//     };
//     Ok(data)
// }

// pub(crate) struct JsonFromRedis {
//     pub(crate) all_count: i64,
//     pub(crate) json: Vec<String>,
//     pub(crate) json_couont: i64,
// }

//
