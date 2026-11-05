use crate::common;
use category_service::redis::json::{del_json, get_json, set_json};
use redis::aio::ConnectionManager;

pub async fn set_get_del_roundtrip(client: ConnectionManager) {
    let mut client = client;
    let key = common::redis::uniq("json");
    let val = serde_json::json!({"a": 1, "b": [true, false], "nested": {"x": "y"}});

    set_json(&mut client.clone(), &key, &val).await.unwrap();
    let got = get_json(&mut client.clone(), &key).await.unwrap().unwrap();
    assert_eq!(got["a"], 1);
    assert_eq!(got["b"][0], true);
    assert_eq!(got["nested"]["x"], "y");

    del_json(&mut client.clone(), &key).await.unwrap();
    assert!(get_json(&mut client, &key).await.unwrap().is_none());
}

pub async fn get_missing_returns_none(client: ConnectionManager) {
    let mut client = client;
    let key = common::redis::uniq("missing");
    assert!(get_json(&mut client, &key).await.unwrap().is_none());
}
