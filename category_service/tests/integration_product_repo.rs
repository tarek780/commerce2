mod common;

use category_service::db::impls::category::CategoryRepo;
use category_service::db::impls::product::{ProductFilter, ProductRepo};
use category_service::error::AppError;

#[tokio::test]
async fn crud_and_get_by_slug() {
    let pool = common::db::setup().await;
    let cat = CategoryRepo::insert(&pool, "C", "c", None).await.unwrap();

    let id = ProductRepo::insert(
        &pool, Some(cat), "Phone", "phone", Some("desc"), Some("Acme"), true,
    ).await.unwrap();

    let row = ProductRepo::get(&pool, id).await.unwrap();
    assert_eq!(row.name, "Phone");
    assert_eq!(row.brand.as_deref(), Some("Acme"));
    assert!(row.is_active);

    assert_eq!(ProductRepo::get_by_slug(&pool, "phone").await.unwrap().id, id);

    // slug conflict
    let err = ProductRepo::insert(&pool, None, "Other", "phone", None, None, true)
        .await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));

    // update multiple fields
    ProductRepo::update(
        &pool, id, Some(Some(cat)), Some("Phone2"), None,
        Some("new desc"), Some("Brand2"), Some(false),
    ).await.unwrap();
    let row = ProductRepo::get(&pool, id).await.unwrap();
    assert_eq!(row.name, "Phone2");
    assert_eq!(row.description.as_deref(), Some("new desc"));
    assert_eq!(row.brand.as_deref(), Some("Brand2"));
    assert!(!row.is_active);

    // delete
    ProductRepo::delete(&pool, id).await.unwrap();
    assert!(matches!(ProductRepo::get(&pool, id).await.unwrap_err(), AppError::NotFound(_)));
}

#[tokio::test]
async fn filters_and_cursor_pagination() {
    let pool = common::db::setup().await;
    let cat = CategoryRepo::insert(&pool, "C", "c", None).await.unwrap();

    let p1 = ProductRepo::insert(&pool, Some(cat), "Phone", "phone", Some("smart"), Some("Acme"), true).await.unwrap();
    let p2 = ProductRepo::insert(&pool, Some(cat), "Laptop", "laptop", None, Some("Other"), false).await.unwrap();

    // by category
    let (rows, _) = ProductRepo::list(&pool, &ProductFilter { category_id: Some(cat), ..Default::default() }, None, 10).await.unwrap();
    assert_eq!(rows.len(), 2);

    // by active
    let (rows, _) = ProductRepo::list(&pool, &ProductFilter { is_active: Some(true), ..Default::default() }, None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, p1);

    // by brand
    let (rows, _) = ProductRepo::list(&pool, &ProductFilter { brand: Some("Other".into()), ..Default::default() }, None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, p2);

    // search ILIKE
    let (rows, _) = ProductRepo::list(&pool, &ProductFilter { search: Some("LAP".into()), ..Default::default() }, None, 10).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, p2);

    // pagination: page size 1
    let (rows, has_more) = ProductRepo::list(&pool, &ProductFilter::default(), None, 1).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(has_more);
    let (rows2, has_more2) = ProductRepo::list(&pool, &ProductFilter::default(), Some(rows[0].id), 10).await.unwrap();
    assert_eq!(rows2.len(), 1);
    assert!(!has_more2);
    assert_eq!(rows2[0].id, p2);
}