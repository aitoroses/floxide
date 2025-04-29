mod helpers;

use floxide_redis::{RedisWorkQueue, RedisClient, RedisConfig};
use floxide_core::distributed::WorkQueue;
use async_trait::async_trait;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct DummyContext;

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
async fn test_work_queue_roundtrip() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let queue = RedisWorkQueue::<DummyWorkItem>::new(client);

    let run_id = "run1";
    let item1 = DummyWorkItem { id: "item1".to_string() };
    let item2 = DummyWorkItem { id: "item2".to_string() };

    // Enqueue
    WorkQueue::<DummyContext, DummyWorkItem>::enqueue(&queue, run_id, item1.clone()).await.expect("enqueue");
    WorkQueue::<DummyContext, DummyWorkItem>::enqueue(&queue, run_id, item2.clone()).await.expect("enqueue");

    // Pending work
    let pending = WorkQueue::<DummyContext, DummyWorkItem>::pending_work(&queue, run_id).await.expect("pending");
    assert_eq!(pending.len(), 2);

    // Dequeue
    let (deq_run_id, deq_item) = WorkQueue::<DummyContext, DummyWorkItem>::dequeue(&queue).await.expect("dequeue").expect("item");
    assert_eq!(deq_run_id, run_id);
    assert!(deq_item == item1 || deq_item == item2);

    // Purge
    WorkQueue::<DummyContext, DummyWorkItem>::purge_run(&queue, run_id).await.expect("purge");
    let pending = WorkQueue::<DummyContext, DummyWorkItem>::pending_work(&queue, run_id).await.expect("pending");
    assert!(pending.is_empty());
    tracing::info!(?pending, "Work queue roundtrip successful");
    redis.cleanup().await;
} 