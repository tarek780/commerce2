use bigdecimal::BigDecimal;
use category_service::db::impls::variant::VariantRow;
use category_service::graphql::types::variant::Variant;
use chrono::Utc;
use std::str::FromStr;
use uuid::Uuid;

pub fn from_row_maps_all_fields() {
    let now = Utc::now();
    let id = Uuid::now_v7();
    let parent_id = Uuid::now_v7();
    let row = VariantRow {
        id: id,
        product_id: parent_id,
        sku: "SKU-1".into(),
        price: BigDecimal::from_str("19.99").unwrap(),
        compare_at_price: Some(BigDecimal::from_str("29.99").unwrap()),
        image_url: Some("http://img".into()),
        attributes: serde_json::json!({"color": "red"}),
        is_active: true,
        created_at: now,
        updated_at: now,
    };
    let v: Variant = row.into();
    assert_eq!(v.id.0, id.to_string());
    assert_eq!(v.product_id.0, parent_id.to_string());
    assert_eq!(v.sku, "SKU-1");
    assert_eq!(v.price, "19.99");
    assert_eq!(v.compare_at_price.as_deref(), Some("29.99"));
    assert_eq!(v.image_url.as_deref(), Some("http://img"));
    assert_eq!(v.attributes["color"], "red");
    assert!(v.is_active);
}

pub fn from_row_handles_optionals_none() {
    let now = Utc::now();
    let id = Uuid::now_v7();
    let parent_id = Uuid::now_v7();
    let row = VariantRow {
        id: id,
        product_id: parent_id,
        sku: "s".into(),
        price: BigDecimal::from_str("1").unwrap(),
        compare_at_price: None,
        image_url: None,
        attributes: serde_json::json!({}),
        is_active: false,
        created_at: now,
        updated_at: now,
    };
    let v: Variant = row.into();
    assert!(v.compare_at_price.is_none());
    assert!(v.image_url.is_none());
    assert!(!v.is_active);
}
