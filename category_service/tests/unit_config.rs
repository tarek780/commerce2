use category_service::config::AppConfig;
use category_service::error::AppError;
use std::sync::Mutex;

// Serialize env-mutating tests inside this binary.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn clear() {
    for k in [
        "DATABASE_URL", "REDIS_URL", "REDIS_STREAM_KEY",
        "REDIS_CONSUMER_GROUP", "REDIS_CONSUMER_NAME",
        "INVENTORY_GRPC_URL", "BIND_ADDRESS",
        "REDIS_INDEX_PRODUCTS", "REDIS_INDEX_CATEGORIES",
    ] {
        unsafe { std::env::remove_var(k) };
    }
}

#[test]
fn missing_required_returns_config_error() {
    let _g = ENV_LOCK.lock().unwrap();
    clear();
    let err = AppConfig::from_env().unwrap_err();
    match err {
        AppError::Config(m) => assert!(m.contains("DATABASE_URL")),
        other => panic!("expected Config, got {other:?}"),
    }
    clear();
}

#[test]
fn defaults_applied() {
    let _g = ENV_LOCK.lock().unwrap();
    clear();
    unsafe {
        std::env::set_var("DATABASE_URL", "postgres://x");
        std::env::set_var("REDIS_URL", "redis://x");
        std::env::set_var("REDIS_STREAM_KEY", "stream");
        std::env::set_var("REDIS_CONSUMER_GROUP", "grp");
        std::env::set_var("INVENTORY_GRPC_URL", "http://inv");
    }
    let cfg = AppConfig::from_env().unwrap();
    assert_eq!(cfg.redis_consumer_name, "worker-1");
    assert_eq!(cfg.bind_addr, "0.0.0.0:8084");
    assert_eq!(cfg.index_products, "catalog:products:idx");
    assert_eq!(cfg.index_categories, "catalog:categories:idx");
    clear();
}

#[test]
fn overrides_applied() {
    let _g = ENV_LOCK.lock().unwrap();
    clear();
    unsafe {
        std::env::set_var("DATABASE_URL", "pg");
        std::env::set_var("REDIS_URL", "r");
        std::env::set_var("REDIS_STREAM_KEY", "s");
        std::env::set_var("REDIS_CONSUMER_GROUP", "g");
        std::env::set_var("INVENTORY_GRPC_URL", "i");
        std::env::set_var("REDIS_CONSUMER_NAME", "worker-7");
        std::env::set_var("BIND_ADDRESS", "127.0.0.1:9999");
    }
    let cfg = AppConfig::from_env().unwrap();
    assert_eq!(cfg.redis_consumer_name, "worker-7");
    assert_eq!(cfg.bind_addr, "127.0.0.1:9999");
    clear();
}