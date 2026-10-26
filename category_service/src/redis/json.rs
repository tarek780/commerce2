use redis::aio::ConnectionManager;
use serde_json::Value;
use crate::error::{AppError, AppResult};

pub async fn set_json(
    mgr: &mut ConnectionManager,
    key: &str,
    value: &Value,
) -> AppResult<()> {
    let payload = serde_json::to_string(value)?;
    let _: () = redis::cmd("JSON.SET")
        .arg(key).arg("$").arg(payload)
        .query_async(mgr).await
        .map_err(|e| AppError::Internal(format!("JSON.SET: {e}")))?;
    Ok(())
}

pub async fn get_json(
    mgr: &mut ConnectionManager,
    key: &str,
) -> AppResult<Option<Value>> {
    let reply: Option<String> = redis::cmd("JSON.GET")
        .arg(key).arg("$")
        .query_async(mgr).await?;
    match reply {
        Some(s) => {
           
            let arr: Vec<Value> = serde_json::from_str(&s)?;
            Ok(arr.into_iter().next())
        }
        None => Ok(None),
    }
}

pub async fn del_json(mgr: &mut ConnectionManager, key: &str) -> AppResult<()> {
    let _: i64 = redis::cmd("JSON.DEL").arg(key).query_async(mgr).await?;
    Ok(())
}