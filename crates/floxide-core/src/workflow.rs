// crates/floxide-core/src/workflow.rs
use async_trait::async_trait;
use crate::{context::WorkflowCtx, error::FloxideError};

#[async_trait]
pub trait Workflow: Send + Sync + 'static {
    type Input: Send + 'static;
    /// Output type returned by the workflow's terminal branch
    type Output: Send + 'static;
    /// Execute the workflow, returning the output of the terminal branch
    async fn run(
        &mut self,
        ctx: &mut WorkflowCtx<()>,
        input: Self::Input
    ) -> Result<Self::Output, FloxideError>;
}
