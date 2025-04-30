mod helpers;

use chrono::Utc;
use floxide_core::distributed::{RunInfo, RunInfoStore, RunStatus};
use floxide_redis::{RedisClient, RedisConfig, RedisRunInfoStore};

#[tokio::test]
async fn test_run_info_store_crud() {
    helpers::init_tracing();
    let mut redis = helpers::TestRedis::start().await;
    let config = RedisConfig::new(redis.redis_url());
    let client = RedisClient::new(config).await.expect("redis client");
    let store = RedisRunInfoStore::new(client);

    let run_id = "run1".to_string();
    let info = RunInfo {
        run_id: run_id.clone(),
        status: RunStatus::Running,
        started_at: Utc::now(),
        finished_at: None,
        output: None,
    };

    // Insert
    store.insert_run(info.clone()).await.expect("insert");
    // Get
    let got = store.get_run(&run_id).await.expect("get").expect("exists");
    assert_eq!(got.run_id, run_id);
    assert_eq!(got.status, RunStatus::Running);

    // Update status
    store
        .update_status(&run_id, RunStatus::Completed)
        .await
        .expect("update status");
    let got = store.get_run(&run_id).await.expect("get").expect("exists");
    assert_eq!(got.status, RunStatus::Completed);

    // Update finished_at
    let now = Utc::now();
    store
        .update_finished_at(&run_id, now)
        .await
        .expect("update finished_at");
    let got = store.get_run(&run_id).await.expect("get").expect("exists");
    assert_eq!(got.finished_at.unwrap().timestamp(), now.timestamp());

    // List
    let all = store.list_runs(None).await.expect("list");
    assert!(all.iter().any(|r| r.run_id == run_id));
    tracing::info!(?all, "Run info store roundtrip successful");
    redis.cleanup().await;
}
