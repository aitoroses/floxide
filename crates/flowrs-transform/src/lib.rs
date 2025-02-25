//! # Flowrs Transform
//!
//! Transform node abstractions for the flowrs framework.
//!
//! This crate provides the `TransformNode` trait and related utilities for working with
//! transformation-oriented workflow nodes that follow a functional programming approach.
//!
//! ## Key Components
//!
//! - `TransformNode`: A trait for nodes that transform input data to output data
//! - `TransformContext`: A simple context wrapper for TransformNode input
//! - `TransformNodeAdapter`: Adapter to convert a TransformNode to a LifecycleNode
//! - Helper functions for creating and converting transform nodes
//!
//! ## Migration from flowrs-async
//!
//! This crate was previously named `flowrs-async` and has been renamed to better
//! reflect its purpose. If you were using `flowrs-async`, update your imports
//! from `flowrs_async` to `flowrs_transform`.

use async_trait::async_trait;
use flowrs_core::{
    error::FlowrsError, lifecycle_node, ActionType, DefaultAction, LifecycleNode, NodeId,
};
use std::fmt::Debug;
use std::marker::PhantomData;
use uuid::Uuid;
use std::sync::Arc;
use futures::future::BoxFuture;

/// A simplified transform node trait for functional data transformations
///
/// The `TransformNode` trait provides a simplified interface for creating nodes
/// that follow a functional transformation pattern with explicit input and output types.
/// Unlike the more general `LifecycleNode`, which operates on a shared context,
/// a `TransformNode` transforms data directly from input to output.
///
/// Each `TransformNode` implements a three-phase lifecycle:
/// 1. `prep`: Validates and prepares the input data
/// 2. `exec`: Performs the main transformation from input to output
/// 3. `post`: Post-processes the output data
///
/// ## Benefits of TransformNode
///
/// - Simpler API focusing on data transformation
/// - Direct error types specific to the node (vs. generic FlowrsError)
/// - Functional programming style with explicit input/output
/// - Easier to compose and reason about
///
/// ## Example
///
/// ```rust
/// use async_trait::async_trait;
/// use flowrs_transform::TransformNode;
/// use std::error::Error;
///
/// // Custom error type
/// #[derive(Debug)]
/// struct MyError(String);
/// impl std::fmt::Display for MyError {
///     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
/// impl Error for MyError {}
///
/// // A transform node that converts strings to uppercase
/// struct UppercaseTransformer;
///
/// #[async_trait]
/// impl TransformNode<String, String, MyError> for UppercaseTransformer {
///     async fn prep(&self, input: String) -> Result<String, MyError> {
///         if input.trim().is_empty() {
///             return Err(MyError("Input cannot be empty".to_string()));
///         }
///         Ok(input)
///     }
///
///     async fn exec(&self, input: String) -> Result<String, MyError> {
///         Ok(input.to_uppercase())
///     }
///
///     async fn post(&self, output: String) -> Result<String, MyError> {
///         Ok(format!("Processed: {}", output))
///     }
/// }
/// ```
#[async_trait]
pub trait TransformNode<Input, Output, Error>: Send + Sync
where
    Input: Send + 'static,
    Output: Send + 'static,
    Error: std::error::Error + Send + Sync + 'static,
{
    /// Preparation phase
    async fn prep(&self, input: Input) -> Result<Input, Error>;

    /// Execution phase
    async fn exec(&self, input: Input) -> Result<Output, Error>;

    /// Post-execution phase
    async fn post(&self, output: Output) -> Result<Output, Error>;
}

/// Adapter to convert a TransformNode to a LifecycleNode
pub struct TransformNodeAdapter<TN, Input, Output, Error, Action>
where
    TN: TransformNode<Input, Output, Error>,
    Input: Clone + Send + Sync + 'static,
    Output: Clone + Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static,
    Action: ActionType + Default + Send + Sync + 'static,
{
    node: TN,
    id: NodeId,
    _phantom: PhantomData<(Input, Output, Error, Action)>,
}

impl<TN, Input, Output, Error, Action> TransformNodeAdapter<TN, Input, Output, Error, Action>
where
    TN: TransformNode<Input, Output, Error>,
    Input: Clone + Send + Sync + 'static,
    Output: Clone + Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static,
    Action: ActionType + Default + Send + Sync + 'static,
{
    /// Create a new adapter for a TransformNode
    pub fn new(node: TN) -> Self {
        Self {
            node,
            id: Uuid::new_v4().to_string(),
            _phantom: PhantomData,
        }
    }

    /// Create a new adapter with a specific ID
    pub fn with_id(node: TN, id: impl Into<String>) -> Self {
        Self {
            node,
            id: id.into(),
            _phantom: PhantomData,
        }
    }
}

impl<TN, Input, Output, Error, Action> Debug for TransformNodeAdapter<TN, Input, Output, Error, Action>
where
    TN: TransformNode<Input, Output, Error> + Debug,
    Input: Clone + Send + Sync + 'static,
    Output: Clone + Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static,
    Action: ActionType + Default + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformNodeAdapter")
            .field("node", &self.node)
            .field("id", &self.id)
            .finish()
    }
}

/// Context wrapper for TransformNode
#[derive(Debug, Clone)]
pub struct TransformContext<Input> {
    pub input: Input,
}

impl<Input> TransformContext<Input> {
    /// Create a new transform context
    pub fn new(input: Input) -> Self {
        Self { input }
    }
}

#[async_trait]
impl<TN, Input, Output, Error, Action> LifecycleNode<TransformContext<Input>, Action>
    for TransformNodeAdapter<TN, Input, Output, Error, Action>
where
    TN: TransformNode<Input, Output, Error> + Send + Sync + 'static,
    Input: Clone + Send + Sync + 'static,
    Output: Clone + Send + Sync + 'static,
    Error: std::error::Error + Send + Sync + 'static + Into<FlowrsError>,
    Action: ActionType + Default + Send + Sync + 'static,
{
    type PrepOutput = Input;
    type ExecOutput = Output;

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn prep(&self, ctx: &mut TransformContext<Input>) -> Result<Self::PrepOutput, FlowrsError> {
        self.node
            .prep(ctx.input.clone())
            .await
            .map_err(|e| e.into())
    }

    async fn exec(&self, prep_result: Self::PrepOutput) -> Result<Self::ExecOutput, FlowrsError> {
        self.node.exec(prep_result).await.map_err(|e| e.into())
    }

    async fn post(
        &self,
        _prep_result: Self::PrepOutput,
        exec_result: Self::ExecOutput,
        _ctx: &mut TransformContext<Input>,
    ) -> Result<Action, FlowrsError> {
        let result = self.node.post(exec_result).await.map_err(|e| e.into())?;
        Ok(Action::default())
    }
}

/// Create a new transform node from closures
pub fn transform_node<P, E, Po, I, O, Err>(
    prep_fn: P,
    exec_fn: E,
    post_fn: Po,
) -> impl TransformNode<I, O, Err>
where
    I: Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
    Err: std::error::Error + Send + Sync + 'static,
    P: Fn(I) -> BoxFuture<'static, Result<I, Err>> + Send + Sync + 'static,
    E: Fn(I) -> BoxFuture<'static, Result<O, Err>> + Send + Sync + 'static,
    Po: Fn(O) -> BoxFuture<'static, Result<O, Err>> + Send + Sync + 'static,
{
    struct ClosureTransformNode<P, E, Po, I, O, Err> {
        prep_fn: P,
        exec_fn: E,
        post_fn: Po,
        _phantom: PhantomData<(I, O, Err)>,
    }

    impl<P, E, Po, I, O, Err> Debug for ClosureTransformNode<P, E, Po, I, O, Err> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("ClosureTransformNode").finish()
        }
    }

    #[async_trait]
    impl<P, E, Po, I, O, Err> TransformNode<I, O, Err> for ClosureTransformNode<P, E, Po, I, O, Err>
    where
        I: Clone + Send + Sync + 'static,
        O: Clone + Send + Sync + 'static,
        Err: std::error::Error + Send + Sync + 'static,
        P: Fn(I) -> BoxFuture<'static, Result<I, Err>> + Send + Sync + 'static,
        E: Fn(I) -> BoxFuture<'static, Result<O, Err>> + Send + Sync + 'static,
        Po: Fn(O) -> BoxFuture<'static, Result<O, Err>> + Send + Sync + 'static,
    {
        async fn prep(&self, input: I) -> Result<I, Err> {
            (self.prep_fn)(input).await
        }

        async fn exec(&self, input: I) -> Result<O, Err> {
            (self.exec_fn)(input).await
        }

        async fn post(&self, output: O) -> Result<O, Err> {
            (self.post_fn)(output).await
        }
    }

    ClosureTransformNode {
        prep_fn,
        exec_fn,
        post_fn,
        _phantom: PhantomData,
    }
}

/// Convert a TransformNode to a LifecycleNode
pub fn to_lifecycle_node<TN, I, O, Err, A>(
    transform_node: TN,
) -> impl LifecycleNode<TransformContext<I>, A, PrepOutput = I, ExecOutput = O>
where
    TN: TransformNode<I, O, Err> + Send + Sync + 'static,
    I: Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
    Err: std::error::Error + Send + Sync + 'static + Into<FlowrsError>,
    A: ActionType + Default + Send + Sync + 'static,
{
    TransformNodeAdapter::<TN, I, O, Err, A>::new(transform_node)
}

/// Create a transform node from closures using the async syntax
pub fn create_transform_node<I, O, Err>(
    prep_fn: impl Fn(I) -> BoxFuture<'static, Result<I, Err>> + Send + Sync + 'static,
    exec_fn: impl Fn(I) -> BoxFuture<'static, Result<O, Err>> + Send + Sync + 'static,
    post_fn: impl Fn(O) -> BoxFuture<'static, Result<O, Err>> + Send + Sync + 'static,
) -> impl TransformNode<I, O, Err>
where
    I: Clone + Send + Sync + 'static,
    O: Clone + Send + Sync + 'static,
    Err: std::error::Error + Send + Sync + 'static,
{
    transform_node(prep_fn, exec_fn, post_fn)
} 