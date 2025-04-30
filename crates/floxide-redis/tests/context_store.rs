mod helpers;

use floxide_core::distributed::context_store::ContextStore;
use floxide_redis::{RedisClient, RedisConfig, RedisContextStore};
use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
struct DummyContext {
    pub value: i32,
}

impl floxide_core::merge::Merge for DummyContext {
    fn merge(&mut self, other: Self) {
        self.value += other.value;
    }
}

#[tokio::test]
async fn test_context_store_set_get_merge() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisContextStore::<DummyContext>::new(client);

    let ctx1 = DummyContext { value: 10 };
    let ctx2 = DummyContext { value: 5 };

    // Set context
    store.set("run1", ctx1.clone()).await.expect("set context");
    // Get context
    let loaded = store
        .get("run1")
        .await
        .expect("get context")
        .expect("context exists");
    assert_eq!(loaded, ctx1);

    // Merge context
    store
        .merge("run1", ctx2.clone())
        .await
        .expect("merge context");
    let merged = store
        .get("run1")
        .await
        .expect("get context")
        .expect("context exists");
    assert_eq!(merged.value, ctx1.value + ctx2.value);

    redis.cleanup().await;
}
