//! Redis configuration for Floxide distributed workflow system.

use std::time::Duration;

/// Configuration for Redis connection.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL (e.g., "redis://127.0.0.1:6379/")
    pub url: String,
    /// Optional username for Redis authentication
    pub username: Option<String>,
    /// Optional password for Redis authentication
    pub password: Option<String>,
    /// Connection timeout in seconds
    pub connection_timeout: Duration,
    /// Key prefix for Redis keys to avoid collisions
    pub key_prefix: String,
}

impl RedisConfig {
    /// Create a new Redis configuration with the given URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            username: None,
            password: None,
            connection_timeout: Duration::from_secs(5),
            key_prefix: "floxide:".to_string(),
        }
    }

    /// Set the username for Redis authentication.
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password for Redis authentication.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set the connection timeout.
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set the key prefix for Redis keys.
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }

    /// Build the Redis connection URL with authentication if provided.
    pub(crate) fn build_connection_url(&self) -> String {
        let mut url = self.url.clone();

        // If URL doesn't have redis:// or rediss:// prefix, add it
        if !url.starts_with("redis://") && !url.starts_with("rediss://") {
            url = format!("redis://{}", url);
        }

        // If URL doesn't end with /, add it
        if !url.ends_with('/') {
            url.push('/');
        }

        url
    }
}
