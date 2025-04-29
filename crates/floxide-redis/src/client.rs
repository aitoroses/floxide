//! Redis client for Floxide distributed workflow system.

use crate::config::RedisConfig;
use redis::{aio::ConnectionManager, Client, RedisError};
use thiserror::Error;

/// Errors that can occur in the Redis client.
#[derive(Debug, Error)]
pub enum RedisClientError {
    #[error("Redis error: {0}")]
    Redis(#[from] RedisError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Other error: {0}")]
    Other(String),
}

/// A Redis client for Floxide distributed workflow system.
#[derive(Clone)]
pub struct RedisClient {
    /// The Redis connection manager
    pub(crate) conn: ConnectionManager,
    /// The Redis configuration
    pub(crate) config: RedisConfig,
}

impl RedisClient {
    /// Create a new Redis client with the given configuration.
    pub async fn new(config: RedisConfig) -> Result<Self, RedisClientError> {
        let url = config.build_connection_url();
        let client = Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        
        Ok(Self {
            conn,
            config,
        })
    }

    /// Get the key prefix for Redis keys.
    pub fn key_prefix(&self) -> &str {
        &self.config.key_prefix
    }

    /// Create a prefixed key for Redis.
    pub fn prefixed_key(&self, key: &str) -> String {
        format!("{}{}", self.key_prefix(), key)
    }
}
