use std::fmt::{self, Display};
use thiserror::Error;

use crate::node::NodeId;

/// All possible errors that can occur in the flowrs framework
#[derive(Error, Debug, Clone)]
pub enum FlowrsError {
    /// Error related to node execution
    #[error("Node execution error: {0}")]
    NodeExecution(String),

    /// Error related to workflow execution
    #[error("Workflow execution error: {0}")]
    WorkflowExecution(String),

    /// Error when a node is not found in the workflow
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Error when an action transition is not defined
    #[error("No transition defined for action: {0}")]
    NoTransitionDefined(String),

    /// Error during serialization
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Error during deserialization
    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    /// Error during async task joining
    #[error("Task join error: {0}")]
    JoinError(String),

    /// A cycle was detected in the workflow
    #[error("Cycle detected in workflow execution")]
    WorkflowCycleDetected,

    /// Error in the workflow definition
    #[error("Workflow definition error: {0}")]
    WorkflowDefinitionError(String),

    /// Error during batch processing
    #[error("Batch processing error: {0}")]
    BatchProcessingError(String),

    /// Node returned an unexpected outcome
    #[error("Unexpected node outcome: {0}")]
    UnexpectedOutcome(String),

    /// Other generic errors
    #[error("{0}")]
    Other(String),
}

/// A specialized Result type for flowrs operations
pub type FlowrsResult<T> = Result<T, FlowrsError>;

impl FlowrsError {
    /// Create a new node execution error
    pub fn node_execution(node_id: impl Display, message: impl Display) -> Self {
        Self::NodeExecution(format!("Node {}: {}", node_id, message))
    }

    /// Create a new batch processing error
    pub fn batch_processing(
        message: impl Display,
        source: Box<dyn std::error::Error + Send + Sync>,
    ) -> Self {
        Self::BatchProcessingError(format!("{}: {}", message, source))
    }

    /// Create a new unexpected outcome error
    pub fn unexpected_outcome(message: impl Display) -> Self {
        Self::UnexpectedOutcome(message.to_string())
    }
}
