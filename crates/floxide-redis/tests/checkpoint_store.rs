mod helpers;

use floxide_redis::{RedisCheckpointStore, RedisClient, RedisConfig};
use floxide_core::checkpoint::{Checkpoint, CheckpointStore};
use std::collections::VecDeque;
use async_trait::async_trait;

// Dummy context and work item for testing
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
struct DummyContext {
    pub value: i32,
}

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
}

#[tokio::test]
async fn test_checkpoint_store_save_and_load() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisCheckpointStore::<DummyContext, DummyWorkItem>::new(client);

    let context = DummyContext { value: 42 };
    let mut queue = VecDeque::new();
    queue.push_back(DummyWorkItem { id: "item1".to_string() });
    queue.push_back(DummyWorkItem { id: "item2".to_string() });
    let checkpoint = Checkpoint::new(context.clone(), queue.clone());

    CheckpointStore::save(&store, "test_workflow", &checkpoint).await.expect("save checkpoint");
    let loaded = CheckpointStore::load(&store, "test_workflow").await.expect("load checkpoint").expect("checkpoint exists");
    assert_eq!(checkpoint.context, loaded.context);
    assert_eq!(checkpoint.queue, loaded.queue);
    tracing::info!(?checkpoint, ?loaded, "Checkpoint roundtrip successful");
    redis.cleanup().await;
} 