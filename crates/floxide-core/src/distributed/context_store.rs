use async_trait::async_trait;
use crate::context::Context;
use crate::merge::Merge;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum ContextStoreError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[async_trait]
pub trait ContextStore<C: Context + Merge + Default>: Send + Sync {
    async fn get(&self, run_id: &str) -> Result<Option<C>, ContextStoreError>;
    async fn set(&self, run_id: &str, ctx: C) -> Result<(), ContextStoreError>;
    async fn merge(&self, run_id: &str, ctx: C) -> Result<(), ContextStoreError>;
}

/// In-memory implementation for testing and local runs.
pub struct InMemoryContextStore<C: Context + Merge + Default> {
    inner: Arc<Mutex<HashMap<String, C>>>,
}

impl<C: Context + Merge + Default> Default for InMemoryContextStore<C> {
    fn default() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }
}

impl<C: Context + Merge + Default> Clone for InMemoryContextStore<C> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

#[async_trait]
impl<C: Context + Merge + Default> ContextStore<C> for InMemoryContextStore<C> {
    async fn get(&self, run_id: &str) -> Result<Option<C>, ContextStoreError> {
        let map = self.inner.lock().await;
        Ok(map.get(run_id).cloned())
    }
    async fn set(&self, run_id: &str, ctx: C) -> Result<(), ContextStoreError> {
        let mut map = self.inner.lock().await;
        map.insert(run_id.to_string(), ctx);
        Ok(())
    }
    async fn merge(&self, run_id: &str, ctx: C) -> Result<(), ContextStoreError> {
        let mut map = self.inner.lock().await;
        map.entry(run_id.to_string())
            .and_modify(|existing| existing.merge(ctx.clone()))
            .or_insert(ctx);
        Ok(())
    }
} 