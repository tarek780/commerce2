mod common;

use category_service::redis::json::set_json;
use category_service::redis::search::search_products;
use redis::aio::ConnectionManager;

async fn create_temp_index(m: &mut ConnectionManager, prefix: &str) -> String {
    let idx = format!("test:idx:{}", uuid::Uuid::now_v7());
    let _ = redis::cmd("FT.CREATE")
        .arg(&idx)
        .arg("ON").arg("JSON")
        .arg("PREFIX").arg("1").arg(prefix)
        .arg("SCHEMA")
        .arg("$.name").arg("AS").arg("name").arg("TEXT")
        .arg("$.category_id").arg("AS").arg("category_id").arg("NUMERIC")
        .arg("$.brand").arg("AS").arg("brand").arg("TAG")
        .arg("$.is_active").arg("AS").arg("is_active").arg("TAG")
        .arg("$.price_min").arg("AS").arg("price_min").arg("NUMERIC")
        .query_async::<()>(m)
        .await;
    idx
}

async fn drop_index(m: &mut ConnectionManager, idx: &str) {
    let _: Result<(), _> = redis::cmd("FT.DROPINDEX")
        .arg(idx).arg("DD")
        .query_async(m)
        .await;
}

#[tokio::test]
#[ignore = "requires Redis Stack (RediSearch + RedisJSON)"]
async fn search_by_category_brand_active_and_price() {
    let mut m = common::redis::setup().await;
    let prefix = common::redis::uniq("searchprod");
    let prefix = format!("{prefix}:");
    let idx = create_temp_index(&mut m, &prefix).await;

    let p1 = format!("{prefix}1");
    let p2 = format!("{prefix}2");
    let p3 = format!("{prefix}3");

    set_json(&mut m, &p1, &serde_json::json!({
        "id": 1, "name": "Phone", "category_id": 5,
        "brand": "Acme", "is_active": "true", "price_min": 10.0,
    })).await.unwrap();
    set_json(&mut m, &p2, &serde_json::json!({
        "id": 2, "name": "Laptop", "category_id": 5,
        "brand": "Other", "is_active": "false", "price_min": 500.0,
    })).await.unwrap();
    set_json(&mut m, &p3, &serde_json::json!({
        "id": 3, "name": "Tablet", "category_id": 6,
        "brand": "Acme", "is_active": "true", "price_min": 200.0,
    })).await.unwrap();

    // small wait for indexing
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let by_cat = search_products(&mut m, &idx, "@category_id:[5 5]", 0, 10).await.unwrap();
    assert_eq!(by_cat.len(), 2);

    let by_brand = search_products(&mut m, &idx, "@brand:{Acme}", 0, 10).await.unwrap();
    assert_eq!(by_brand.len(), 2);

    let active = search_products(&mut m, &idx, "@is_active:{true}", 0, 10).await.unwrap();
    assert_eq!(active.len(), 2);

    let price = search_products(&mut m, &idx, "@price_min:[100 +inf]", 0, 10).await.unwrap();
    assert_eq!(price.len(), 2);

    drop_index(&mut m, &idx).await;
}

#[tokio::test]
#[ignore = "requires Redis Stack (RediSearch + RedisJSON)"]
async fn search_limit_and_offset() {
    let mut m = common::redis::setup().await;
    let prefix = common::redis::uniq("paged");
    let prefix = format!("{prefix}:");
    let idx = create_temp_index(&mut m, &prefix).await;

    for i in 0..5 {
        let key = format!("{prefix}{i}");
        set_json(&mut m, &key, &serde_json::json!({
            "id": i, "name": format!("Item{i}"), "category_id": 1,
            "brand": "B", "is_active": "true", "price_min": 1.0,
        })).await.unwrap();
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let page1 = search_products(&mut m, &idx, "@category_id:[1 1]", 0, 2).await.unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = search_products(&mut m, &idx, "@category_id:[1 1]", 2, 2).await.unwrap();
    assert_eq!(page2.len(), 2);

    let ids1: Vec<i64> = page1.iter().filter_map(|d| d["id"].as_i64()).collect();
    let ids2: Vec<i64> = page2.iter().filter_map(|d| d["id"].as_i64()).collect();
    for id in &ids1 {
        assert!(!ids2.contains(id), "pages must not overlap");
    }

    drop_index(&mut m, &idx).await;
}