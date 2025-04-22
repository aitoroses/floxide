use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum FloxideError {
    #[error("Generic error: {0}")]
    Generic(String),
    /// The workflow was cancelled via its cancellation token.
    #[error("Workflow cancelled")]
    Cancelled,
    /// The workflow timed out after the specified duration.
    #[error("Workflow timed out after {0:?}")]
    Timeout(Duration),
} 