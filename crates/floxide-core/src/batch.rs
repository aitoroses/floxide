use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Semaphore;
use tracing::debug;
use uuid::Uuid;

use crate::action::ActionType;
use crate::error::FloxideError;
use crate::node::{Node, NodeId, NodeOutcome};
use crate::workflow::{Workflow, WorkflowError};

/// Trait for contexts that support batch processing
pub trait BatchContext<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Get the items to process in batch
    fn get_batch_items(&self) -> Result<Vec<T>, FloxideError>;

    /// Create a context for a single item
    fn create_item_context(&self, item: T) -> Result<Self, FloxideError>
    where
        Self: Sized;

    /// Update the main context with results from item processing
    fn update_with_results(
        &mut self,
        results: &[Result<T, FloxideError>],
    ) -> Result<(), FloxideError>;
}

/// A node that processes a batch of items in parallel
pub struct BatchNode<Context, ItemType, A = crate::action::DefaultAction>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
{
    id: NodeId,
    item_workflow: Workflow<Context, A>,
    parallelism: usize,
    _phantom: PhantomData<(Context, ItemType, A)>,
}

impl<Context, ItemType, A> BatchNode<Context, ItemType, A>
where
    Context: BatchContext<ItemType> + Clone + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
{
    /// Create a new batch node with the given item workflow and parallelism
    pub fn new(item_workflow: Workflow<Context, A>, parallelism: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            item_workflow,
            parallelism,
            _phantom: PhantomData,
        }
    }
}

impl<Context, ItemType, A> Debug for BatchNode<Context, ItemType, A>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchNode")
            .field("id", &self.id)
            .field("parallelism", &self.parallelism)
            .finish()
    }
}

#[async_trait]
impl<Context, ItemType, A> Node<Context, A> for BatchNode<Context, ItemType, A>
where
    Context: BatchContext<ItemType> + Clone + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Default + Debug + Clone + Send + Sync + 'static,
{
    type Output = Vec<Result<ItemType, FloxideError>>;

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, A>, FloxideError> {
        // Get items to process
        debug!(node_id = %self.id, "Getting batch items to process");
        let items = ctx.get_batch_items()?;
        debug!(node_id = %self.id, item_count = items.len(), "Processing batch items");

        let mut results = Vec::with_capacity(items.len());

        // Create a bounded semaphore to limit concurrency
        let semaphore = Arc::new(Semaphore::new(self.parallelism));
        let mut handles = Vec::with_capacity(items.len());

        // Create tasks for each item
        for item in items {
            let semaphore = semaphore.clone();
            let workflow = self.item_workflow.clone();
            let ctx_clone = ctx.clone();

            // Clone the item for use in the context
            let item_clone = item.clone();

            // Spawn a task for each item
            let handle = tokio::spawn(async move {
                // Acquire a permit from the semaphore to limit concurrency
                let _permit = semaphore.acquire().await.unwrap();

                match ctx_clone.create_item_context(item_clone) {
                    Ok(mut item_ctx) => match workflow.execute(&mut item_ctx).await {
                        Ok(_) => Ok(item),
                        Err(e) => Err(FloxideError::batch_processing(
                            "Failed to process item",
                            Box::new(e),
                        )),
                    },
                    Err(e) => Err(e),
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(FloxideError::JoinError(e.to_string()))),
            }
        }

        // Update the context with results
        ctx.update_with_results(&results)?;

        // Return results
        Ok(NodeOutcome::Success(results))
    }
}

/// BatchFlow provides parallel processing of items from a context
pub struct BatchFlow<Context, ItemType, A = crate::action::DefaultAction>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
{
    id: NodeId,
    batch_node: BatchNode<Context, ItemType, A>,
}

impl<Context, ItemType, A> BatchFlow<Context, ItemType, A>
where
    Context: BatchContext<ItemType> + Clone + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Default + Debug + Clone + Send + Sync + 'static,
{
    /// Create a new batch flow with the given item workflow
    pub fn new(item_workflow: Workflow<Context, A>, parallelism: usize) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            batch_node: BatchNode::new(item_workflow, parallelism),
        }
    }

    /// Execute the batch flow with the given context
    pub async fn execute(
        &self,
        ctx: &mut Context,
    ) -> Result<Vec<Result<ItemType, FloxideError>>, WorkflowError> {
        match self.batch_node.process(ctx).await {
            Ok(NodeOutcome::Success(results)) => Ok(results),
            Ok(_) => Err(WorkflowError::NodeExecution(
                FloxideError::unexpected_outcome("Expected Success outcome from BatchNode"),
            )),
            Err(e) => Err(WorkflowError::NodeExecution(e)),
        }
    }
}

impl<Context, ItemType, A> Debug for BatchFlow<Context, ItemType, A>
where
    Context: BatchContext<ItemType> + Send + Sync + 'static,
    ItemType: Clone + Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatchFlow")
            .field("id", &self.id)
            .field("batch_node", &self.batch_node)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::DefaultAction;
    use crate::node::closure::node;

    // Test context implementation for batch processing
    #[derive(Debug, Clone)]
    struct TestBatchContext {
        items: Vec<i32>,
        results: Vec<Result<i32, FloxideError>>,
    }

    // i32 already implements Clone, so we can use it with our updated BatchContext trait
    impl BatchContext<i32> for TestBatchContext {
        fn get_batch_items(&self) -> Result<Vec<i32>, FloxideError> {
            Ok(self.items.clone())
        }

        fn create_item_context(&self, item: i32) -> Result<Self, FloxideError> {
            Ok(TestBatchContext {
                items: vec![item],
                results: Vec::new(),
            })
        }

        fn update_with_results(
            &mut self,
            results: &[Result<i32, FloxideError>],
        ) -> Result<(), FloxideError> {
            self.results = results.to_vec();
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_batch_node_processing() {
        // Create a simple workflow for processing individual items
        let item_workflow = Workflow::new(node(|mut ctx: TestBatchContext| async move {
            // Double the value of the item
            let item = ctx.items[0] * 2;
            ctx.items = vec![item];
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        }));

        // Create a batch node using this workflow
        let batch_node = BatchNode::new(item_workflow, 4);

        // Create a test context with items to process
        let mut ctx = TestBatchContext {
            items: vec![1, 2, 3, 4, 5],
            results: Vec::new(),
        };

        // Process the batch
        let result = batch_node.process(&mut ctx).await.unwrap();

        // Verify results
        match result {
            NodeOutcome::Success(results) => {
                assert_eq!(results.len(), 5);
                assert!(results.iter().all(|r| r.is_ok()));
            }
            _ => panic!("Expected Success outcome"),
        }
    }

    #[tokio::test]
    async fn test_batch_flow_execution() {
        // Create a simple workflow for processing individual items
        let item_workflow = Workflow::new(node(|mut ctx: TestBatchContext| async move {
            // Double the value of the item
            let item = ctx.items[0] * 2;
            ctx.items = vec![item];
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        }));

        // Create a batch flow
        let batch_flow = BatchFlow::new(item_workflow, 4);

        // Create a test context with items to process
        let mut ctx = TestBatchContext {
            items: vec![1, 2, 3, 4, 5],
            results: Vec::new(),
        };

        // Execute the batch flow
        let results = batch_flow.execute(&mut ctx).await.unwrap();

        // Verify results
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
