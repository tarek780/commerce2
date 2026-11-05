use crate::common::TestContext;
use auth_service::db::user_repo;
use auth_service::error::AppError;
use std::sync::Arc;
pub async fn create(pool: &sqlx::PgPool, email: &str) -> auth_service::models::user::User {
    user_repo::create_user(pool, email, "hash", Some("D"), "Full", "u", "user")
        .await
        .expect("create user")
}

pub async fn create_user_returns_full_row(ctx: Arc<TestContext>) {
    let u = create(&ctx.pool, "create@example.com").await;
    assert_eq!(u.email, "create@example.com");
    assert!(u.email_verified_at.is_none());
    assert!(u.created_at <= chrono::Utc::now());
}

//
pub async fn duplicate_email_maps_to_conflict(ctx: Arc<TestContext>) {
    create(&ctx.pool, "dup2@example.com").await;
    let err = user_repo::create_user(&ctx.pool, "dup2@example.com", "h", None, "F", "u", "user")
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::Conflict(_)));
}

pub async fn find_by_id_and_update_display_name(ctx: Arc<TestContext>) {
    let u = create(&ctx.pool, "id@example.com").await;
    let found = user_repo::find_by_id(&ctx.pool, &u.id.to_string().as_str())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, u.id);

    let updated =
        user_repo::update_display_name(&ctx.pool, &u.id.to_string().as_str(), Some("Renamed"))
            .await
            .unwrap();
    assert_eq!(updated.display_name.as_deref(), Some("Renamed"));
}

pub async fn mark_email_verified_sets_timestamp(ctx: Arc<TestContext>) {
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

pub async fn delete_user_removes_row(ctx: Arc<TestContext>) {
    let u = create(&ctx.pool, "del@example.com").await;
    user_repo::delete(&ctx.pool, &u.id.to_string().as_str())
        .await
        .unwrap();
    assert!(
        user_repo::find_by_id(&ctx.pool, &u.id.to_string().as_str())
            .await
            .unwrap()
            .is_none()
    );
}
