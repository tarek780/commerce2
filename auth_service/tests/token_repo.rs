mod common;

use chrono::{Duration, Utc};
use uuid::Uuid;

use auth_service::db::{token_repo, user_repo};
use common::TestContext;

async fn make_user(pool: &sqlx::PgPool) -> String {
    let u = user_repo::create_user(
        pool,
        "repo@example.com",
        "hash",
        Some("Repo"),
        "Repo Full",
        "repo",
        "user",
    )
    .await
    .unwrap();
    u.id
}

#[tokio::test]
async fn insert_and_find_token() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let uid = make_user(&ctx.pool).await;
    let fam = Uuid::now_v7().to_string();
    let exp = Utc::now() + Duration::hours(1);

    token_repo::insert_refresh_token(&ctx.pool, uid.clone(), "hash-1", fam.clone(), exp)
        .await
        .unwrap();

    let row = token_repo::find_by_hash(&ctx.pool, "hash-1")
        .await
        .unwrap()
        .expect("row must exist");
    assert_eq!(row.user_id, uid);
    assert_eq!(row.family_id, fam);
    assert!(!row.rotated);
    assert!(!row.revoked);
}

#[tokio::test]
async fn mark_rotated_sets_flag() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let uid = make_user(&ctx.pool).await;
    let fam = Uuid::now_v7().to_string();
    let exp = Utc::now() + Duration::hours(1);
    token_repo::insert_refresh_token(&ctx.pool, uid, "h", fam, exp)
        .await
        .unwrap();

    let row = token_repo::find_by_hash(&ctx.pool, "h")
        .await
        .unwrap()
        .unwrap();
    token_repo::mark_rotated(&ctx.pool, &row.id).await.unwrap();

    let row2 = token_repo::find_by_hash(&ctx.pool, "h")
        .await
        .unwrap()
        .unwrap();
    assert!(row2.rotated);
}

#[tokio::test]
async fn revoke_family_revokes_all() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let uid = make_user(&ctx.pool).await;
    let fam = Uuid::now_v7().to_string();
    let exp = Utc::now() + Duration::hours(1);

    for i in 0..3 {
        token_repo::insert_refresh_token(
            &ctx.pool,
            uid.clone(),
            &format!("h{i}"),
            fam.clone(),
            exp,
        )
        .await
        .unwrap();
    }

    token_repo::revoke_family(&ctx.pool, &fam).await.unwrap();

    for i in 0..3 {
        let row = token_repo::find_by_hash(&ctx.pool, &format!("h{i}"))
            .await
            .unwrap()
            .unwrap();
        assert!(row.revoked);
    }
}

#[tokio::test]
async fn delete_removes_tokens_for_user() {
    let ctx = TestContext::new().await;
    common::truncate_all(&ctx.pool).await;

    let uid = make_user(&ctx.pool).await;
    token_repo::insert_refresh_token(
        &ctx.pool,
        uid.clone(),
        "tok",
        Uuid::now_v7().to_string(),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();

    token_repo::delete(&ctx.pool, &uid).await.unwrap();
    assert!(
        token_repo::find_by_hash(&ctx.pool, "tok")
            .await
            .unwrap()
            .is_none()
    );
}
