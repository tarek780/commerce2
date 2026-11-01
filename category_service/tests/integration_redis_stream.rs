mod common;

use category_service::redis::stream::{CatalogEvent, publish};
use redis::AsyncCommands;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires Redis (streams)"]
async fn publish_appends_to_stream() {
    let mut m = common::redis::setup().await;
    let stream = common::redis::uniq("stream");
    let new_id = Uuid::now_v7();
    let id = publish(
        &mut m,
        &stream,
        &CatalogEvent::ProductCreated {
            id: new_id,
            payload: json!({"name": "Phone", "slug": "phone"}),
        },
    )
    .await
    .unwrap();
    assert!(!id.is_empty());
    let id = Uuid::parse_str(&id.to_string().as_str()).expect("valid uuid");

    let len: i64 = m.xlen(&stream).await.unwrap();
    assert_eq!(len, 1);

    // read back
    let entries: Vec<(String, Vec<(String, String)>)> = m.xrange(&stream, "-", "+").await.unwrap();
    assert_eq!(entries.len(), 1);
    let (_, fields) = &entries[0];
    let kind = fields
        .iter()
        .find(|(k, _)| k == "kind")
        .map(|(_, v)| v.clone());
    assert_eq!(kind.as_deref(), Some("product_created"));

    // cleanup
    let _: i64 = m.del(&stream).await.unwrap();
}

#[tokio::test]
#[ignore = "requires Redis (streams)"]
async fn consumer_group_roundtrip() {
    let mut m = common::redis::setup().await;
    let stream = common::redis::uniq("stream");
    let group = "g1";
    let consumer = "c1";

    let _: () = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&stream)
        .arg(group)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(&mut m)
        .await
        .expect("");

    publish(
        &mut m,
        &stream,
        &CatalogEvent::CategoryCreated {
            id: Uuid::now_v7(),
            payload: json!({}),
        },
    )
    .await
    .unwrap();
    publish(
        &mut m,
        &stream,
        &CatalogEvent::ProductDeleted { id: Uuid::now_v7() },
    )
    .await
    .unwrap();

    let reply: redis::streams::StreamReadReply = redis::cmd("XREADGROUP")
        .arg("GROUP")
        .arg(group)
        .arg(consumer)
        .arg("COUNT")
        .arg(10)
        .arg("STREAMS")
        .arg(&stream)
        .arg(">")
        .query_async(&mut m)
        .await
        .unwrap();

    let mut total = 0;
    for key in reply.keys {
        total += key.ids.len();
    }
    assert_eq!(total, 2);

    let _: i64 = m.del(&stream).await.unwrap();
}
