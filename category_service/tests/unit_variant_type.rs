use bigdecimal::BigDecimal;
use category_service::db::impls::variant::VariantRow;
use category_service::graphql::types::variant::Variant;
use chrono::Utc;
use std::str::FromStr;

#[test]
fn from_row_maps_all_fields() {
    let now = Utc::now();
    let row = VariantRow {
        id: 5,
        product_id: 2,
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
    assert_eq!(v.id.0, "5");
    assert_eq!(v.product_id.0, "2");
    assert_eq!(v.sku, "SKU-1");
    assert_eq!(v.price, "19.99");
    assert_eq!(v.compare_at_price.as_deref(), Some("29.99"));
    assert_eq!(v.image_url.as_deref(), Some("http://img"));
    assert_eq!(v.attributes["color"], "red");
    assert!(v.is_active);
}

#[test]
fn from_row_handles_optionals_none() {
    let now = Utc::now();
    let row = VariantRow {
        id: 1,
        product_id: 1,
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