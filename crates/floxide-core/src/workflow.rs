// crates/floxide-core/src/workflow.rs
use async_trait::async_trait;
use crate::{context::WorkflowCtx, error::FloxideError};

#[async_trait]
pub trait Workflow: Send + Sync + 'static {
    type Input: Send + 'static;
    async fn run(
        &mut self,
        ctx: &mut WorkflowCtx<()>,
        input: Self::Input
    ) -> Result<(), FloxideError>;
}
