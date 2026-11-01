mod common;

use category_service::db::impls::category::CategoryRepo;
use category_service::error::AppError;
use uuid::Uuid;

#[tokio::test]
async fn crud_flow() {
    let pool = common::db::setup().await;

    let id = CategoryRepo::insert(&pool, "Electronics", "electronics", None)
        .await
        .unwrap();
    let row = CategoryRepo::get(&pool, id).await.unwrap();
    if let Some(row) = row {
        assert_eq!(row.name, "Electronics");
        assert_eq!(row.slug, "electronics");
        assert!(row.parent_id.is_none());
    } else {
        panic!("Category not found after insert");
    };

    // slug conflict
    let err = CategoryRepo::insert(&pool, "Other", "electronics", None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));

    // update name
    CategoryRepo::update(
        &pool,
        &id.to_string().as_str(),
        Some("Electronics 2"),
        None,
        None,
    )
    .await
    .unwrap();
    let x = CategoryRepo::get(&pool, id).await.unwrap().unwrap().name;
    assert_eq!(x, "Electronics 2");

    // not found on update
    let err = CategoryRepo::update(
        &pool,
        &Uuid::now_v7().to_string().as_str(),
        Some("x"),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));

    // delete
    CategoryRepo::delete(&pool, &id.to_string().as_str())
        .await
        .unwrap();
    assert!(matches!(
        CategoryRepo::get(&pool, id).await.unwrap_err(),
        AppError::NotFound(_)
    ));

    // not found on delete
    let err = CategoryRepo::delete(&pool, &Uuid::now_v7().to_string().as_str())
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
}

#[tokio::test]
async fn children_and_limits() {
    let pool = common::db::setup().await;
    let parent = CategoryRepo::insert(&pool, "P", "p", None).await.unwrap();
    CategoryRepo::insert(&pool, "C1", "c1", Some(&parent.to_string().as_str()))
        .await
        .unwrap();
    CategoryRepo::insert(&pool, "C2", "c2", Some(parent.to_string().as_str()))
        .await
        .unwrap();

    let children = CategoryRepo::list(&pool, Some(parent), 10, 0)
        .await
        .unwrap();
    assert_eq!(children.len(), 2);

    let all = CategoryRepo::list(&pool, None, 10, 0).await.unwrap();
    assert_eq!(all.len(), 3);

    // pagination
    let first = CategoryRepo::list(&pool, None, 2, 0).await.unwrap();
    let second = CategoryRepo::list(&pool, None, 2, 2).await.unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 1);
}
