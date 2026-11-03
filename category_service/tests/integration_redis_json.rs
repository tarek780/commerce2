mod common;

use category_service::redis::json::{del_json, get_json, set_json};
use redis::aio::ConnectionManager;

#[tokio::test]
#[ignore = "requires Redis Stack (RedisJSON)"]
async fn set_get_del_roundtrip() {
    let mut m = common::redis::setup().await;
    let key = common::redis::uniq("json");
    let val = serde_json::json!({"a": 1, "b": [true, false], "nested": {"x": "y"}});

    let mut client = match ConnectionManager::new(m.0).await {
        Ok(o) => o,
        Err(e) => {
            panic!("err : {e}");
            return;
        }
    };

    set_json(&mut client.clone(), &key, &val).await.unwrap();
    let got = get_json(&mut client.clone(), &key).await.unwrap().unwrap();
    assert_eq!(got["a"], 1);
    assert_eq!(got["b"][0], true);
    assert_eq!(got["nested"]["x"], "y");

    del_json(&mut client.clone(), &key).await.unwrap();
    assert!(get_json(&mut client, &key).await.unwrap().is_none());
}

#[tokio::test]
#[ignore = "requires Redis Stack (RedisJSON)"]
async fn get_missing_returns_none() {
    let mut m = common::redis::setup().await;

    let key = common::redis::uniq("missing");
    let mut client = ConnectionManager::new(m.0).await.unwrap();
    assert!(get_json(&mut client, &key).await.unwrap().is_none());
}
