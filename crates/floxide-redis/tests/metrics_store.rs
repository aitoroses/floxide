mod helpers;

use floxide_core::distributed::{MetricsStore, RunMetrics};
use floxide_redis::{RedisClient, RedisConfig, RedisMetricsStore};

#[tokio::test]
async fn test_metrics_store_roundtrip() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisMetricsStore::new(client);

    let run_id = "run1";
    let metrics = RunMetrics {
        total_work_items: 10,
        completed: 5,
        failed: 2,
        retries: 1,
    };

    // Update
    store
        .update_metrics(run_id, metrics.clone())
        .await
        .expect("update");
    // Get
    let got = store
        .get_metrics(run_id)
        .await
        .expect("get")
        .expect("exists");
    assert_eq!(got.total_work_items, 10);
    assert_eq!(got.completed, 5);
    assert_eq!(got.failed, 2);
    assert_eq!(got.retries, 1);
    tracing::info!(?got, "Metrics store roundtrip successful");
    redis.cleanup().await;
}
