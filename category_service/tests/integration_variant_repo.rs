mod common;

use bigdecimal::BigDecimal;
use category_service::db::impls::category::CategoryRepo;
use category_service::db::impls::product::ProductRepo;
use category_service::db::impls::variant::VariantRepo;
use category_service::error::AppError;
use std::str::FromStr;

#[tokio::test]
async fn insert_list_delete_and_sku_conflict() {
    let pool = common::db::setup().await;
    let cat = CategoryRepo::insert(&pool, "C", "c", None).await.unwrap();
    let p = ProductRepo::insert(&pool, Some(cat), "P", "p", None, None, true).await.unwrap();

    let id = VariantRepo::insert(
        &pool, p, "SKU-1",
        BigDecimal::from_str("9.99").unwrap(),
        Some(BigDecimal::from_str("19.99").unwrap()),
        Some("http://img"),
        &serde_json::json!({"size": "M"}),
        true,
    ).await.unwrap();

    let variants = VariantRepo::list_by_product(&pool, p).await.unwrap();
    assert_eq!(variants.len(), 1);
    assert_eq!(variants[0].id, id);
    assert_eq!(variants[0].sku, "SKU-1");

    let err = VariantRepo::insert(
        &pool, p, "SKU-1",
        BigDecimal::from_str("1").unwrap(),
        None, None, &serde_json::json!({}), true,
    ).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));

    VariantRepo::delete(&pool, id).await.unwrap();
    assert!(VariantRepo::list_by_product(&pool, p).await.unwrap().is_empty());

    let err = VariantRepo::delete(&pool, id).await.unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
}

#[tokio::test]
async fn cascade_delete_on_product_removal() {
    let pool = common::db::setup().await;
    let cat = CategoryRepo::insert(&pool, "C", "c", None).await.unwrap();
    let p = ProductRepo::insert(&pool, Some(cat), "P", "p", None, None, true).await.unwrap();
    VariantRepo::insert(&pool, p, "SKU-1",
        BigDecimal::from_str("1").unwrap(),
        None, None, &serde_json::json!({}), true,
    ).await.unwrap();

    ProductRepo::delete(&pool, p).await.unwrap();
    assert!(VariantRepo::list_by_product(&pool, p).await.unwrap().is_empty());
}