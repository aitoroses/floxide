use async_trait::async_trait;
use uuid::Uuid;

use crate::action::ActionType;
use crate::error::FlowrsError;

/// Unique identifier for a node in a workflow
pub type NodeId = String;

/// Possible outcomes of node execution
#[derive(Debug, Clone)]
pub enum NodeOutcome<Output, Action> {
    /// Node execution completed successfully with output
    Success(Output),
    /// Node execution was skipped
    Skipped,
    /// Node completed but wants to route to a different next node
    RouteToAction(Action),
}

/// Core trait that all workflow nodes must implement
#[async_trait]
pub trait Node<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Self::Output: Send + Sync + 'static,
{
    /// The output type produced by this node
    type Output;

    /// Get the unique identifier for this node
    fn id(&self) -> NodeId;

    /// Process the node asynchronously
    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FlowrsError>;
}

/// Module for creating nodes from functions
pub mod node {
    use std::fmt::Debug;
    use std::future::Future;
    use std::marker::PhantomData;

    use super::*;

    /// Create a node from a closure
    pub fn node<Closure, Context, Action, Output, Fut>(closure: Closure) -> ClosureNode<Closure, Context, Action, Output>
    where
        Context: Clone + Send + Sync + 'static,
        Action: ActionType + Send + Sync + 'static,
        Output: Send + Sync + 'static,
        Closure: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(Context, NodeOutcome<Output, Action>), FlowrsError>> + Send + 'static,
    {
        ClosureNode {
            id: Uuid::new_v4().to_string(),
            closure,
            _phantom: PhantomData,
        }
    }

    /// A node implementation that wraps a closure
    #[derive(Clone)]
    pub struct ClosureNode<Closure, Context, Action, Output> {
        id: NodeId,
        closure: Closure,
        _phantom: PhantomData<(Context, Action, Output)>,
    }

    impl<Closure, Context, Action, Output> Debug for ClosureNode<Closure, Context, Action, Output> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ClosureNode")
                .field("id", &self.id)
                .finish()
        }
    }

    #[async_trait]
    impl<Closure, Context, Action, Output, Fut> Node<Context, Action> for ClosureNode<Closure, Context, Action, Output>
    where
        Context: Clone + Send + Sync + 'static,
        Action: ActionType + Send + Sync + 'static,
        Output: Send + Sync + 'static,
        Closure: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(Context, NodeOutcome<Output, Action>), FlowrsError>> + Send + 'static,
    {
        type Output = Output;

        fn id(&self) -> NodeId {
            self.id.clone()
        }

        async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, Action>, FlowrsError> {
            // Clone the context and pass ownership to the closure
            let ctx_clone = ctx.clone();
            
            // Execute the closure, which returns both updated context and outcome
            let (updated_ctx, outcome) = (self.closure)(ctx_clone).await?;
            
            // Update the original context with the updated version
            *ctx = updated_ctx;
            
            // Return just the outcome
            Ok(outcome)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::DefaultAction;

    #[derive(Debug, Clone)]
    struct TestContext {
        value: i32,
    }

    #[tokio::test]
    async fn test_create_node_from_closure() {
        let test_node = node::node(|mut ctx: TestContext| async move {
            ctx.value += 1;
            let value = ctx.value; // Store the value before moving ctx
            Ok((ctx, NodeOutcome::<i32, DefaultAction>::Success(value)))
        });

        let mut context = TestContext { value: 5 };
        let result = test_node.process(&mut context).await.unwrap();

        match result {
            NodeOutcome::Success(value) => {
                assert_eq!(value, 6);
                assert_eq!(context.value, 6);
            }
            _ => panic!("Expected Success outcome"),
        }
    }

    #[tokio::test]
    async fn test_skip_node() {
        let skip_node = node::node(|ctx: TestContext| async move {
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Skipped))
        });

        let mut context = TestContext { value: 5 };
        let result = skip_node.process(&mut context).await.unwrap();

        match result {
            NodeOutcome::Skipped => {}
            _ => panic!("Expected Skipped outcome"),
        }

        // Value should remain unchanged
        assert_eq!(context.value, 5);
    }

    #[tokio::test]
    async fn test_route_to_action() {
        let route_node = node::node(|ctx: TestContext| async move {
            Ok((ctx, NodeOutcome::<(), DefaultAction>::RouteToAction(DefaultAction::Custom("alternate_path".into()))))
        });

        let mut context = TestContext { value: 5 };
        let result = route_node.process(&mut context).await.unwrap();

        match result {
            NodeOutcome::RouteToAction(action) => {
                assert_eq!(action.name(), "alternate_path");
            }
            _ => panic!("Expected RouteToAction outcome"),
        }
    }
} 