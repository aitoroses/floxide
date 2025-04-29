mod helpers;

use floxide_redis::{RedisErrorStore, RedisClient, RedisConfig};
use floxide_core::distributed::{ErrorStore, WorkflowError};
use chrono::Utc;

#[tokio::test]
async fn test_error_store_roundtrip() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisErrorStore::new(client);

    let run_id = "run1";
    let error = WorkflowError {
        work_item: "item1".to_string(),
        error: "something went wrong".to_string(),
        attempt: 1,
        timestamp: Utc::now(),
    };

    // Record
    store.record_error(run_id, error.clone()).await.expect("record");
    // Get
    let got = store.get_errors(run_id).await.expect("get");
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].work_item, "item1");
    assert_eq!(got[0].error, "something went wrong");
    tracing::info!(?got, "Error store roundtrip successful");
    redis.cleanup().await;
} 