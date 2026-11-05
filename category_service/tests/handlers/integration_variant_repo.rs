use bigdecimal::BigDecimal;
use category_service::db::impls::category::CategoryRepo;
use category_service::db::impls::product::ProductRepo;
use category_service::db::impls::variant::VariantRepo;
use category_service::error::AppError;
use sqlx::{Pool, Postgres};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

pub async fn insert_list_delete_and_sku_conflict(pool: Arc<Pool<Postgres>>) {
    let cat = CategoryRepo::insert(&pool, "C", "c", None).await.unwrap();
    let p = ProductRepo::insert(&pool, Some(cat), "P", "p", None, None, true)
        .await
        .unwrap();
    let pid = Uuid::parse_str(p.as_str()).expect("");
    let id = VariantRepo::insert(
        &pool,
        &pid.to_string().as_str(),
        "SKU-1",
        BigDecimal::from_str("9.99").unwrap(),
        Some(BigDecimal::from_str("19.99").unwrap()),
        Some("http://img"),
        &serde_json::json!({"size": "M"}),
        true,
    )
    .await
    .unwrap();
    let idd = Uuid::parse_str(&id.to_string().as_str()).unwrap();

    let variants = VariantRepo::list_by_product(&pool, pid).await.unwrap();
    assert_eq!(variants.len(), 1);
    assert_eq!(variants[0].id, idd);
    assert_eq!(variants[0].sku, "SKU-1");

    let err = VariantRepo::insert(
        &pool,
        &pid.to_string().as_str(),
        "SKU-1",
        BigDecimal::from_str("1").unwrap(),
        None,
        None,
        &serde_json::json!({}),
        true,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));

    VariantRepo::delete(&pool, idd).await.unwrap();
    assert!(
        VariantRepo::list_by_product(&pool, pid)
            .await
            .unwrap()
            .is_empty()
    );

    let err = VariantRepo::delete(&pool, idd).await.unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
}

pub async fn cascade_delete_on_product_removal(pool: Arc<Pool<Postgres>>) {
    let cat = CategoryRepo::insert(&pool, "C", "c", None).await.unwrap();
    let p = ProductRepo::insert(&pool, Some(cat), "P", "p", None, None, true)
        .await
        .unwrap();
    let pid = Uuid::parse_str(p.as_str()).expect("");
    VariantRepo::insert(
        &pool,
        &pid.to_string().as_str(),
        "SKU-1",
        BigDecimal::from_str("1").unwrap(),
        None,
        None,
        &serde_json::json!({}),
        true,
    )
    .await
    .unwrap();

    ProductRepo::delete(&pool, pid).await.unwrap();
    assert!(
        VariantRepo::list_by_product(&pool, pid)
            .await
            .unwrap()
            .is_empty()
    );
}
