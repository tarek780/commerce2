#![allow(dead_code)]

use std::sync::Arc;

use inventory_service::inventory::repo::InventoryRepo;
use inventory_service::inventory::service::InventorySvc;
use inventory_service::pb::StockUpdate;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;

use testcontainers::GenericImage;
use testcontainers::{
    core::{ImageExt, WaitFor},
    runners::AsyncRunner,
};
use tokio::io::AsyncWriteExt;
use tokio::io::stdout;
use uuid::Uuid;

pub struct TestEnv {
    pub pool: PgPool,
    pub repo: Arc<InventoryRepo>,
    pub stock_tx: broadcast::Sender<StockUpdate>,
    pub svc: InventorySvc,
}

pub async fn build_env() -> Arc<TestEnv> {
    dotenvy::dotenv().ok();

    let pg_image = GenericImage::new("postgres", "16.15-alpine3.24")
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_USER", "myuser")
        .with_env_var("POSTGRES_PASSWORD", "mypassword")
        .with_env_var("POSTGRES_DB", "mydb");

    let pg_container = pg_image
        .start()
        .await
        .expect("Failed to start Postgres container");

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let pg_port = match pg_container.get_host_port_ipv4(5432).await {
        Ok(c) => c,
        Err(e) => {
            let _ = stdout()
                .write_all(format!("Failed to get Postgres container port: {}\n", e).as_bytes())
                .await;
            panic!("Failed to get Postgres container port: {}", e);
        }
    };

    // let database_url =
    //     format!("postgres://myuser:mypassword@127.0.0.1:{pg_port}/mydb?sslmode=disable");

    let database_url = format!(
        "postgres://myuser:mypassword@127.0.0.1:5433/mydb?sslmode=disable
"
    );

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect test postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let (stock_tx, _) = broadcast::channel(1024);
    let repo = Arc::new(InventoryRepo::new(pool.clone()));
    let svc = InventorySvc::new(repo.clone(), stock_tx.clone());

    Arc::new(TestEnv {
        pool,
        repo,
        stock_tx,
        svc,
    })
}

// #[macro_export]
// macro_rules! require_db {
//     () => {
//         match common::build_env().await {
//             Some(env) => env,
//             None => {
//                 eprintln!("skip: DATABASE_URL not set or DB unreachable");
//                 return;
//             }
//         }
//     };
// }

pub fn new_variant_id() -> i32 {
    // 1..=2_000_000_000
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as i64;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    ((secs.wrapping_mul(1_000_000) ^ nanos) % 2_000_000_000 + 1) as i32
}

pub async fn seed_inventory(pool: &PgPool, variant_id: Uuid, stock: i32) {
    sqlx::query(
        r#"INSERT INTO inventory (id ,variant_id, stock_quantity, reserved_quantity)
           VALUES ($1, $2, $3, 0)
           ON CONFLICT (variant_id) DO UPDATE
           SET stock_quantity = EXCLUDED.stock_quantity, reserved_quantity = 0"#,
    )
    .bind(Uuid::now_v7())
    .bind(variant_id)
    .bind(stock)
    .execute(pool)
    .await
    .expect("seed inventory");
}

pub async fn cleanup_variant(pool: &PgPool, variant_id: Uuid) {
    let _ = sqlx::query("DELETE FROM reservations WHERE variant_id = $1")
        .bind(variant_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM inventory_logs WHERE sku = $1")
        .bind(variant_id.to_string())
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM inventory WHERE variant_id = $1")
        .bind(variant_id)
        .execute(pool)
        .await;
}

pub fn new_reservation_key() -> String {
    uuid::Uuid::now_v7().to_string()
}
