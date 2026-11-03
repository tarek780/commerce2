use category_service::redis::stream::CatalogEvent;
use serde_json::json;
use uuid::Uuid;
#[test]
fn kinds() {
    let id = Uuid::now_v7();
    assert_eq!(
        CatalogEvent::CategoryCreated {
            id,
            payload: json!({})
        }
        .kind(),
        "category_created"
    );
    assert_eq!(
        CatalogEvent::CategoryUpdated {
            id,
            payload: json!({})
        }
        .kind(),
        "category_updated"
    );
    assert_eq!(
        CatalogEvent::CategoryDeleted { id }.kind(),
        "category_deleted"
    );
    assert_eq!(
        CatalogEvent::ProductCreated {
            id,
            payload: json!({})
        }
        .kind(),
        "product_created"
    );
    assert_eq!(
        CatalogEvent::ProductUpdated {
            id,
            payload: json!({})
        }
        .kind(),
        "product_updated"
    );
    assert_eq!(
        CatalogEvent::ProductDeleted { id }.kind(),
        "product_deleted"
    );
}

#[test]
fn ids() {
    let id1 = Uuid::now_v7();
    let id12 = Uuid::now_v7();
    assert_eq!(
        CatalogEvent::CategoryCreated {
            id: id1,
            payload: json!({})
        }
        .id(),
        id1
    );
    assert_eq!(CatalogEvent::ProductDeleted { id: id12 }.id(), id12);
}

#[test]
fn serde_tagged_roundtrip() {
    let id = Uuid::now_v7();
    let ev = CatalogEvent::ProductUpdated {
        id,
        payload: json!({"name": "X"}),
    };
    let s = serde_json::to_string(&ev).unwrap();
    assert!(s.contains("\"kind\":\"product_updated\""));
    let back: CatalogEvent = serde_json::from_str(&s).unwrap();
    assert_eq!(back.id(), id);
    assert_eq!(back.kind(), "product_updated");
}
