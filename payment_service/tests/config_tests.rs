mod common;

use payment_service::config::PaymentConfig;

#[tokio::test]
#[serial_test::serial]
async fn config_from_env_reads_all_fields() {
    let envs = [
        ("DATABASE_URL", "postgres://x"),
        ("GRPC_ADDR", "0.0.0.0:50051"),
        ("KAFKA_BROKERS", "k:9092"),
        ("KAFKA_PAYMENT_TOPIC", "topic"),
        ("EXPIRY_SCAN_SECS", "30"),
        ("PAYMENT_GATEWAY", "mock"),
        ("GATEWAY_PUBLIC_BASE", "http://base"),
        ("HTTP_ADDR", "0.0.0.0:8080"),
        ("RESERVATION_TTL_SECS", "600"),
        ("WEBHOOK_SECRET", "sec"),
    ];
    for (k, v) in envs {
        unsafe {
            std::env::set_var(k, v);
        }
    }

    let cfg = PaymentConfig::from_env().await.expect("from_env failed");

    assert_eq!(cfg.database_url, "postgres://x");
    assert_eq!(cfg.grpc_addr, "0.0.0.0:50051");
    assert_eq!(cfg.expiry_scan_secs, 30);
    assert_eq!(cfg.reservation_ttl_secs, 600);
    assert_eq!(cfg.gateway, "mock");
    assert_eq!(cfg.webhook_secret, "sec");
}

#[tokio::test]
#[serial_test::serial]
async fn config_from_env_fails_when_missing() {
    unsafe {
        std::env::remove_var("DATABASE_URL");
    }

    let res = PaymentConfig::from_env().await;
    assert!(res.is_err(), "should fail when DATABASE_URL missing");
}
