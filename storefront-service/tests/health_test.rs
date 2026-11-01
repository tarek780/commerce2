mod common;
use common::get_json;

#[tokio::test]
async fn health_returns_ok() {
    let h = common::harness().await;
    let (status, json, _) = get_json(&h.app, "/health", &[]).await;
    assert_eq!(status, 200);
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn ready_returns_ok() {
    let h = common::harness().await;
    let (status, json, _) = get_json(&h.app, "/ready", &[]).await;
    assert_eq!(status, 200);
    assert_eq!(json["status"], "ready");
}