//! Redis implementation for Floxide distributed workflow system.
//!
//! This crate provides Redis-backed implementations of the distributed system
//! interfaces defined in floxide-core, including:
//!
//! - `RedisWorkQueue`: A Redis-backed work queue for distributed workflow execution
//! - `RedisCheckpointStore`: A Redis-backed checkpoint store for workflow state persistence
//! - `RedisRunInfoStore`: A Redis-backed store for workflow run metadata
//! - `RedisMetricsStore`: A Redis-backed store for workflow metrics
//! - `RedisErrorStore`: A Redis-backed store for workflow errors
//! - `RedisLivenessStore`: A Redis-backed store for worker liveness tracking
//! - `RedisWorkItemStateStore`: A Redis-backed store for work item state tracking

mod checkpoint_store;
mod client;
mod config;
mod error_store;
mod liveness_store;
mod metrics_store;
mod run_info_store;
mod work_item_store;
mod work_queue;

pub use checkpoint_store::RedisCheckpointStore;
pub use client::{RedisClient, RedisClientError};
pub use config::RedisConfig;
pub use error_store::RedisErrorStore;
pub use liveness_store::RedisLivenessStore;
pub use metrics_store::RedisMetricsStore;
pub use run_info_store::RedisRunInfoStore;
pub use work_item_store::RedisWorkItemStateStore;
pub use work_queue::RedisWorkQueue;
