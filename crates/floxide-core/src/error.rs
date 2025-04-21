use thiserror::Error;

#[derive(Debug, Error)]
pub enum FloxideError {
    #[error("Generic error: {0}")]
    Generic(String),
} 