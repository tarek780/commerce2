use category_service::redis::stream::CatalogEvent;
use serde_json::json;

#[test]
fn kinds() {
    assert_eq!(CatalogEvent::CategoryCreated { id: 1, payload: json!({}) }.kind(), "category_created");
    assert_eq!(CatalogEvent::CategoryUpdated { id: 1, payload: json!({}) }.kind(), "category_updated");
    assert_eq!(CatalogEvent::CategoryDeleted { id: 1 }.kind(), "category_deleted");
    assert_eq!(CatalogEvent::ProductCreated { id: 1, payload: json!({}) }.kind(), "product_created");
    assert_eq!(CatalogEvent::ProductUpdated { id: 1, payload: json!({}) }.kind(), "product_updated");
    assert_eq!(CatalogEvent::ProductDeleted { id: 1 }.kind(), "product_deleted");
}

#[test]
fn ids() {
    assert_eq!(CatalogEvent::CategoryCreated { id: 5, payload: json!({}) }.id(), 5);
    assert_eq!(CatalogEvent::ProductDeleted { id: 9 }.id(), 9);
}

#[test]
fn serde_tagged_roundtrip() {
    let ev = CatalogEvent::ProductUpdated {
        id: 3,
        payload: json!({"name": "X"}),
    };
    let s = serde_json::to_string(&ev).unwrap();
    assert!(s.contains("\"kind\":\"product_updated\""));
    let back: CatalogEvent = serde_json::from_str(&s).unwrap();
    assert_eq!(back.id(), 3);
    assert_eq!(back.kind(), "product_updated");
}