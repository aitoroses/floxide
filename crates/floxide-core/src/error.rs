use thiserror::Error;
use std::time::Duration;

#[derive(Debug, Error, Clone)]
pub enum FloxideError {
    #[error("Generic error: {0}")]
    Generic(String),
    /// The workflow was cancelled via its cancellation token.
    #[error("Workflow cancelled")]
    Cancelled,
    /// The workflow timed out after the specified duration.
    #[error("Workflow timed out after {0:?}")]
    Timeout(Duration),
    /// The workflow was never started, so cannot be resumed.
    #[error("Workflow has not been started")] 
    NotStarted,
    /// The workflow has already completed; no more work to resume.
    #[error("Workflow already completed")] 
    AlreadyCompleted,
} 