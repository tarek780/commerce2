use crate::common::{TestHarness, get_json};
use std::sync::Arc;

pub async fn health_returns_ok(h: &Arc<TestHarness>) {
    let (status, json, _) = get_json("/health", &[]).await;
    assert_eq!(status, 200);
    assert_eq!(json["status"], "ok");
}

pub async fn ready_returns_ok(h: &Arc<TestHarness>) {
    let (status, json, _) = get_json("/ready", &[]).await;
    assert_eq!(status, 200);
    assert_eq!(json["status"], "ready");
}

pub async fn health_master(state: Arc<TestHarness>) {
    let h = state;
    health_returns_ok(&h).await;
    ready_returns_ok(&h).await;
}
