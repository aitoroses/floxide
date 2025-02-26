//! # Floxide Core
//!
//! Core components of the floxide framework for directed graph workflows

// Modules
pub mod action;
pub mod batch;
pub mod error;
pub mod lifecycle;
mod node;
mod retry;
mod workflow;

// Re-exports
pub use action::{ActionType, DefaultAction};
pub use batch::{BatchContext, BatchFlow, BatchNode};
pub use error::{FloxideError, FloxideResult};
pub use lifecycle::{lifecycle_node, LifecycleNode};
pub use node::closure::node;
pub use node::{Node, NodeId, NodeOutcome};
pub use retry::{BackoffStrategy, RetryNode};
pub use workflow::{Workflow, WorkflowError};

// Testing module
#[cfg(test)]
mod tests {
    mod action_type;
    mod node;
    mod workflow;
}
