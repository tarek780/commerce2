use crate::common;
use category_service::redis::stream::{CatalogEvent, publish};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde_json::json;
use uuid::Uuid;
pub async fn publish_appends_to_stream(client: ConnectionManager) {
    let mut client = client;
    let stream = common::redis::uniq("stream");
    let new_id = Uuid::now_v7();
    let id = publish(
        &mut client,
        &stream,
        &CatalogEvent::ProductCreated {
            id: new_id,
            payload: json!({"name": "Phone", "slug": "phone"}),
        },
    )
    .await
    .unwrap();
    assert!(!id.is_empty());
    //let id = Uuid::parse_str(&id.to_string().as_str()).expect("valid uuid");

    let len: i64 = client.xlen(&stream).await.unwrap();
    assert_eq!(len, 1);

    // read back
    let entries: Vec<(String, Vec<(String, String)>)> =
        client.xrange(&stream, "-", "+").await.unwrap();
    assert_eq!(entries.len(), 1);
    let (_, fields) = &entries[0];
    let kind = fields
        .iter()
        .find(|(k, _)| k == "kind")
        .map(|(_, v)| v.clone());
    assert_eq!(kind.as_deref(), Some("product_created"));

    // cleanup
    let _: i64 = client.del(&stream).await.unwrap();
}

pub async fn consumer_group_roundtrip(client: ConnectionManager) {
    let mut client = client;

    let stream = common::redis::uniq("stream");
    let group = "g1";
    let consumer = "c1";

    let _: () = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(&stream)
        .arg(group)
        .arg("$")
        .arg("MKSTREAM")
        .query_async(&mut client)
        .await
        .expect("");

    publish(
        &mut client,
        &stream,
        &CatalogEvent::CategoryCreated {
            id: Uuid::now_v7(),
            payload: json!({}),
        },
    )
    .await
    .unwrap();
    publish(
        &mut client,
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
        .query_async(&mut client)
        .await
        .unwrap();

    let mut total = 0;
    for key in reply.keys {
        total += key.ids.len();
    }
    assert_eq!(total, 2);

    let _: i64 = client.del(&stream).await.unwrap();
}
