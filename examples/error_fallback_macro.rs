// examples/error_fallback_macro.rs
// Demonstrates workflow-level error fallback using the `on_failure` clause in edges.

use floxide_macros::workflow;
use floxide_core::*;
use async_trait::async_trait;

/// A node that always returns an error
#[derive(Clone, Debug)]
pub struct AlwaysFailNode;

#[async_trait]
impl Node for AlwaysFailNode {
    type Input = ();
    type Output = String;

    async fn process(
        &self,
        _ctx: &(),
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        // Intentionally fail
        println!("AlwaysFailNode: intentional failure");
        Err(FloxideError::Generic("intentional failure".into()))
    }
}

/// A node that recovers from failure
#[derive(Clone, Debug)]
pub struct RecoveryNode;

#[async_trait]
impl Node for RecoveryNode {
    type Input = ();
    type Output = String;

    async fn process(
        &self,
        _ctx: &(),
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        // On recovery, produce a default value
        println!("RecoveryNode: recovery");
        Ok(Transition::Next("recovered".into()))
    }
}

// Define a workflow with a failure fallback edge
workflow! {
    pub struct ErrorFallbackWorkflow {
        fail: AlwaysFailNode,
        recovery: RecoveryNode,
    }
    start = fail;
    edges {
        // no success successors for `fail`
        fail => {};
        // on failure, route to `recovery`
        fail on_failure => { [ recovery ] };
        // `recovery` is terminal
        recovery => {};
    }
}

/// Runs the error fallback workflow and returns the output string
pub async fn run_error_fallback_workflow() -> Result<String, Box<dyn std::error::Error>> {
    let wf = ErrorFallbackWorkflow {
        fail: AlwaysFailNode,
        recovery: RecoveryNode,
    };
    let ctx = WorkflowCtx::new(());
    // Run the workflow: `fail` will error, then fallback to `recovery`
    let output: String = wf.run(&ctx, ()).await?;
    Ok(output)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let output = run_error_fallback_workflow().await?;
    println!("Workflow output after fallback: {}", output);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_error_fallback_workflow() {
        let output = run_error_fallback_workflow().await.expect("workflow should succeed after fallback");
        assert_eq!(output, "recovered");
    }
}