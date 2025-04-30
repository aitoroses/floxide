mod helpers;

use async_trait::async_trait;
use floxide_core::distributed::{WorkItemStateStore, WorkItemStatus};
use floxide_redis::{RedisClient, RedisConfig, RedisWorkItemStateStore};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct DummyWorkItem {
    pub id: String,
}

impl std::fmt::Display for DummyWorkItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

#[async_trait]
impl floxide_core::workflow::WorkItem for DummyWorkItem {
    fn instance_id(&self) -> String {
        self.id.clone()
    }
    fn is_terminal(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn test_work_item_store_status_and_attempts() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisWorkItemStateStore::<DummyWorkItem>::new(client);

    let item = DummyWorkItem {
        id: "item1".to_string(),
    };
    let run_id = "run1";

    // Set status
    store
        .set_status(run_id, &item, WorkItemStatus::InProgress)
        .await
        .expect("set status");
    let status = store.get_status(run_id, &item).await.expect("get status");
    assert_eq!(status, WorkItemStatus::InProgress);

    // Increment attempts
    let attempts = store
        .increment_attempts(run_id, &item)
        .await
        .expect("inc attempts");
    assert_eq!(attempts, 1);
    let attempts = store
        .increment_attempts(run_id, &item)
        .await
        .expect("inc attempts");
    assert_eq!(attempts, 2);

    // Reset attempts
    store
        .reset_attempts(run_id, &item)
        .await
        .expect("reset attempts");
    let attempts = store
        .get_attempts(run_id, &item)
        .await
        .expect("get attempts");
    assert_eq!(attempts, 0);

    // Get all
    let all = store.get_all(run_id).await.expect("get all");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].work_item, item);
    tracing::info!(?all, "Work item state roundtrip successful");
    redis.cleanup().await;
}
