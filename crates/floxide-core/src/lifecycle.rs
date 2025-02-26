use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;

use async_trait::async_trait;
use tracing::debug;
use uuid::Uuid;

use crate::action::ActionType;
use crate::error::FloxideError;
use crate::node::{Node, NodeId, NodeOutcome};

/// A node that implements the prep/exec/post lifecycle
#[async_trait]
pub trait LifecycleNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Self::PrepOutput: Clone + Send + Sync + 'static,
    Self::ExecOutput: Clone + Send + Sync + 'static,
{
    /// Output type from the preparation phase
    type PrepOutput;

    /// Output type from the execution phase
    type ExecOutput;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;

    /// Preparation phase - perform setup and validation
    async fn prep(&self, ctx: &mut Context) -> Result<Self::PrepOutput, FloxideError>;

    /// Execution phase - perform the main work
    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FloxideError>;

    /// Post-execution phase - determine the next action and update context
    async fn post(
        &self,
        prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        ctx: &mut Context,
    ) -> Result<Action, FloxideError>;
}

/// Adapter to convert a LifecycleNode to a standard Node
pub struct LifecycleNodeAdapter<LN, Context, Action>
where
    LN: LifecycleNode<Context, Action>,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    inner: LN,
    _phantom: PhantomData<(Context, Action)>,
}

impl<LN, Context, Action> LifecycleNodeAdapter<LN, Context, Action>
where
    LN: LifecycleNode<Context, Action>,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    /// Create a new lifecycle node adapter
    pub fn new(inner: LN) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<LN, Context, Action> Debug for LifecycleNodeAdapter<LN, Context, Action>
where
    LN: LifecycleNode<Context, Action> + Debug,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecycleNodeAdapter")
            .field("inner", &self.inner)
            .finish()
    }
}

#[async_trait]
impl<LN, Context, Action> Node<Context, Action> for LifecycleNodeAdapter<LN, Context, Action>
where
    LN: LifecycleNode<Context, Action> + Send + Sync + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    LN::ExecOutput: Send + Sync + 'static,
{
    type Output = LN::ExecOutput;

    fn id(&self) -> NodeId {
        self.inner.id()
    }

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FloxideError> {
        // Run the three-phase lifecycle
        debug!(node_id = %self.id(), "Starting prep phase");
        let prep_result = self.inner.prep(ctx).await?;

        debug!(node_id = %self.id(), "Starting exec phase");
        let exec_result = self.inner.exec(prep_result.clone()).await?;

        debug!(node_id = %self.id(), "Starting post phase");
        let next_action = self
            .inner
            .post(prep_result, exec_result.clone(), ctx)
            .await?;

        // Return the appropriate outcome based on the action
        Ok(NodeOutcome::RouteToAction(next_action))
    }
}

/// Convenience function to create a lifecycle node from closures
pub fn lifecycle_node<
    PrepFn,
    ExecFn,
    PostFn,
    Context,
    Action,
    PrepOut,
    ExecOut,
    PrepFut,
    ExecFut,
    PostFut,
>(
    id: Option<String>,
    prep_fn: PrepFn,
    exec_fn: ExecFn,
    post_fn: PostFn,
) -> impl Node<Context, Action, Output = ExecOut>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    PrepOut: Send + Sync + Clone + 'static,
    ExecOut: Send + Sync + Clone + 'static,
    PrepFn: Fn(&mut Context) -> PrepFut + Send + Sync + 'static,
    ExecFn: Fn(PrepOut) -> ExecFut + Send + Sync + 'static,
    PostFn: Fn(PrepOut, ExecOut, &mut Context) -> PostFut + Send + Sync + 'static,
    PrepFut: Future<Output = Result<PrepOut, FloxideError>> + Send + 'static,
    ExecFut: Future<Output = Result<ExecOut, FloxideError>> + Send + 'static,
    PostFut: Future<Output = Result<Action, FloxideError>> + Send + 'static,
{
    struct ClosureLifecycleNode<P, E, Po, Ctx, Act, PO, EO> {
        id: NodeId,
        prep_fn: P,
        exec_fn: E,
        post_fn: Po,
        _phantom: PhantomData<(Ctx, Act, PO, EO)>,
    }

    impl<P, E, Po, Ctx, Act, PO, EO> Debug for ClosureLifecycleNode<P, E, Po, Ctx, Act, PO, EO> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ClosureLifecycleNode")
                .field("id", &self.id)
                .finish()
        }
    }

    #[async_trait]
    impl<P, E, Po, Ctx, Act, PO, EO, PF, EF, PoF> LifecycleNode<Ctx, Act>
        for ClosureLifecycleNode<P, E, Po, Ctx, Act, PO, EO>
    where
        Ctx: Send + Sync + 'static,
        Act: ActionType + Send + Sync + 'static,
        PO: Send + Sync + Clone + 'static,
        EO: Send + Sync + Clone + 'static,
        P: Fn(&mut Ctx) -> PF + Send + Sync + 'static,
        E: Fn(PO) -> EF + Send + Sync + 'static,
        Po: Fn(PO, EO, &mut Ctx) -> PoF + Send + Sync + 'static,
        PF: Future<Output = Result<PO, FloxideError>> + Send + 'static,
        EF: Future<Output = Result<EO, FloxideError>> + Send + 'static,
        PoF: Future<Output = Result<Act, FloxideError>> + Send + 'static,
    {
        type PrepOutput = PO;
        type ExecOutput = EO;

        fn id(&self) -> NodeId {
            self.id.clone()
        }

        async fn prep(&self, ctx: &mut Ctx) -> Result<Self::PrepOutput, FloxideError> {
            (self.prep_fn)(ctx).await
        }

        async fn exec(
            &self,
            prep_result: Self::PrepOutput,
        ) -> Result<Self::ExecOutput, FloxideError> {
            (self.exec_fn)(prep_result).await
        }

        async fn post(
            &self,
            prep_result: Self::PrepOutput,
            exec_result: Self::ExecOutput,
            ctx: &mut Ctx,
        ) -> Result<Act, FloxideError> {
            (self.post_fn)(prep_result, exec_result, ctx).await
        }
    }

    let node_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let lifecycle_node = ClosureLifecycleNode {
        id: node_id,
        prep_fn,
        exec_fn,
        post_fn,
        _phantom: PhantomData,
    };

    LifecycleNodeAdapter::new(lifecycle_node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::DefaultAction;

    #[derive(Debug, Clone)]
    struct TestContext {
        value: i32,
        path: Vec<String>,
    }

    // A simple lifecycle node for testing
    struct TestLifecycleNode {
        id: NodeId,
    }

    #[async_trait]
    impl LifecycleNode<TestContext, DefaultAction> for TestLifecycleNode {
        type PrepOutput = i32;
        type ExecOutput = i32;

        fn id(&self) -> NodeId {
            self.id.clone()
        }

        async fn prep(&self, ctx: &mut TestContext) -> Result<Self::PrepOutput, FloxideError> {
            ctx.path.push("prep".to_string());
            Ok(ctx.value)
        }

        async fn exec(
            &self,
            prep_result: Self::PrepOutput,
        ) -> Result<Self::ExecOutput, FloxideError> {
            Ok(prep_result * 2)
        }

        async fn post(
            &self,
            _prep_result: Self::PrepOutput,
            exec_result: Self::ExecOutput,
            ctx: &mut TestContext,
        ) -> Result<DefaultAction, FloxideError> {
            ctx.path.push("post".to_string());
            ctx.value = exec_result;
            Ok(DefaultAction::Next)
        }
    }

    // A lifecycle node that errors during exec
    struct ErrorLifecycleNode {
        id: NodeId,
    }

    #[async_trait]
    impl LifecycleNode<TestContext, DefaultAction> for ErrorLifecycleNode {
        type PrepOutput = i32;
        type ExecOutput = i32;

        fn id(&self) -> NodeId {
            self.id.clone()
        }

        async fn prep(&self, _ctx: &mut TestContext) -> Result<Self::PrepOutput, FloxideError> {
            Ok(42)
        }

        async fn exec(
            &self,
            _prep_result: Self::PrepOutput,
        ) -> Result<Self::ExecOutput, FloxideError> {
            Err(FloxideError::node_execution("test", "Simulated error"))
        }

        async fn post(
            &self,
            _prep_result: Self::PrepOutput,
            _exec_result: Self::ExecOutput,
            _ctx: &mut TestContext,
        ) -> Result<DefaultAction, FloxideError> {
            // This shouldn't be called
            Ok(DefaultAction::Next)
        }
    }

    #[tokio::test]
    async fn test_lifecycle_node() {
        let lifecycle_node = TestLifecycleNode {
            id: "test-node".to_string(),
        };
        let node = LifecycleNodeAdapter::new(lifecycle_node);

        let mut ctx = TestContext {
            value: 5,
            path: Vec::new(),
        };

        let result = node.process(&mut ctx).await.unwrap();

        match result {
            NodeOutcome::RouteToAction(action) => {
                assert_eq!(action, DefaultAction::Next);
            }
            _ => panic!("Expected RouteToAction outcome"),
        }

        assert_eq!(ctx.value, 10); // 5 * 2 = 10
        assert_eq!(ctx.path, vec!["prep", "post"]);
    }

    #[tokio::test]
    async fn test_lifecycle_node_with_error() {
        let lifecycle_node = ErrorLifecycleNode {
            id: "error-node".to_string(),
        };
        let node = LifecycleNodeAdapter::new(lifecycle_node);

        let mut ctx = TestContext {
            value: 5,
            path: Vec::new(),
        };

        let result = node.process(&mut ctx).await;
        assert!(result.is_err());
    }
}
