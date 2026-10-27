mod common;

use auth_service::db::user_repo;
use auth_service::error::AppError;
use common::TestContext;

async fn create(pool: &sqlx::PgPool, email: &str) -> auth_service::models::user::User {
    user_repo::create_user(pool, email, "hash", Some("D"), "Full", "u", "user")
        .await
        .expect("create user")
}

#[tokio::test]
async fn create_user_returns_full_row() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let u = create(&ctx.pool, "create@example.com").await;
    assert_eq!(u.email, "create@example.com");
    assert!(u.email_verified_at.is_none());
    assert!(u.created_at <= chrono::Utc::now());
}

#[tokio::test]
async fn duplicate_email_maps_to_conflict() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    create(&ctx.pool, "dup2@example.com").await;
    let err = user_repo::create_user(&ctx.pool, "dup2@example.com", "h", None, "F", "u", "user")
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::Conflict(_)));
}

#[tokio::test]
async fn find_by_id_and_update_display_name() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let u = create(&ctx.pool, "id@example.com").await;
    let found = user_repo::find_by_id(&ctx.pool, &u.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, u.id);

    let updated = user_repo::update_display_name(&ctx.pool, &u.id, Some("Renamed"))
        .await
        .unwrap();
    assert_eq!(updated.display_name.as_deref(), Some("Renamed"));
}

#[tokio::test]
async fn mark_email_verified_sets_timestamp() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let u = create(&ctx.pool, "mark@example.com").await;
    let affected = user_repo::mark_email_verified(&ctx.pool, &u.email)
        .await
        .unwrap();
    assert_eq!(affected, 1);

    let again = user_repo::mark_email_verified(&ctx.pool, "nobody@example.com")
        .await
        .unwrap();
    assert_eq!(again, 0);
}

#[tokio::test]
async fn delete_user_removes_row() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let u = create(&ctx.pool, "del@example.com").await;
    user_repo::delete(&ctx.pool, &u.id).await.unwrap();
    assert!(
        user_repo::find_by_id(&ctx.pool, &u.id)
            .await
            .unwrap()
            .is_none()
    );
}
