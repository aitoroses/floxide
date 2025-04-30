mod helpers;

use chrono::Utc;
use floxide_core::distributed::{LivenessStore, WorkerHealth, WorkerStatus};
use floxide_redis::{RedisClient, RedisConfig, RedisLivenessStore};

#[tokio::test]
async fn test_liveness_store_roundtrip() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisLivenessStore::new(client);

    let worker_id = 42;
    let now = Utc::now();
    // Update heartbeat
    store
        .update_heartbeat(worker_id, now)
        .await
        .expect("update heartbeat");
    let hb = store
        .get_heartbeat(worker_id)
        .await
        .expect("get heartbeat")
        .expect("exists");
    assert_eq!(hb.timestamp(), now.timestamp());

    // Update health
    let mut health = WorkerHealth::default();
    health.worker_id = worker_id;
    health.status = WorkerStatus::InProgress;
    store
        .update_health(worker_id, health.clone())
        .await
        .expect("update health");
    let got = store
        .get_health(worker_id)
        .await
        .expect("get health")
        .expect("exists");
    assert_eq!(got.worker_id, worker_id);
    assert!(matches!(got.status, WorkerStatus::InProgress));

    // List workers
    let workers = store.list_workers().await.expect("list workers");
    assert!(workers.contains(&worker_id));
    // List health
    let healths = store.list_health().await.expect("list health");
    assert!(healths.iter().any(|h| h.worker_id == worker_id));
    tracing::info!(?healths, "Liveness store roundtrip successful");
    redis.cleanup().await;
}
