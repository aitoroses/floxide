//! Support for long-running processes with checkpoints in the Floxide framework.
//!
//! This crate provides the `LongRunningNode` trait and related implementations for
//! handling long-running processes that need to be suspended and resumed over time.

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use floxide_core::{ActionType, DefaultAction, FloxideError, Node, NodeId, NodeOutcome};
use serde::{Deserialize, Serialize};

/// Represents the outcome of a long-running process.
#[derive(Debug)]
pub enum LongRunningOutcome<T, S> {
    /// Processing is complete with result
    Complete(T),
    /// Processing needs to be suspended with saved state
    Suspend(S),
}

/// Trait for nodes that handle long-running processes with checkpoints.
///
/// A `LongRunningNode` is capable of processing work incrementally, saving its state
/// between executions, and resuming from the last checkpoint.
#[async_trait]
pub trait LongRunningNode<Context, Action>: Send + Sync
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    Self::State: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    Self::Output: Send + 'static,
{
    /// Type representing the node's processing state
    type State;

    /// Type representing the final output
    type Output;

    /// Process the next step, potentially suspending execution
    async fn process(
        &self,
        state: Option<Self::State>,
        ctx: &mut Context,
    ) -> Result<LongRunningOutcome<Self::Output, Self::State>, FloxideError>;

    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}

/// Extension trait for ActionType to support long-running specific actions
pub trait LongRunningActionExt: ActionType {
    /// Create a suspend action for long-running nodes
    fn suspend() -> Self;

    /// Create a resume action for long-running nodes
    fn resume() -> Self;

    /// Create a complete action for long-running nodes
    fn complete() -> Self;

    /// Check if this is a suspend action
    fn is_suspend(&self) -> bool;

    /// Check if this is a resume action
    fn is_resume(&self) -> bool;

    /// Check if this is a complete action
    fn is_complete(&self) -> bool;
}

// Implement the extension trait for DefaultAction
impl LongRunningActionExt for DefaultAction {
    fn suspend() -> Self {
        DefaultAction::Custom("suspend".to_string())
    }

    fn resume() -> Self {
        DefaultAction::Custom("resume".to_string())
    }

    fn complete() -> Self {
        DefaultAction::Custom("complete".to_string())
    }

    fn is_suspend(&self) -> bool {
        matches!(self, DefaultAction::Custom(s) if s == "suspend")
    }

    fn is_resume(&self) -> bool {
        matches!(self, DefaultAction::Custom(s) if s == "resume")
    }

    fn is_complete(&self) -> bool {
        matches!(self, DefaultAction::Custom(s) if s == "complete")
    }
}

/// A simple long-running node that uses a closure for processing.
pub struct SimpleLongRunningNode<Context, Action, State, Output, F>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    State: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Option<State>, &mut Context) -> Result<LongRunningOutcome<Output, State>, FloxideError>
        + Send
        + Sync
        + 'static,
{
    id: NodeId,
    process_fn: Arc<F>,
    _phantom: std::marker::PhantomData<(Context, Action, State, Output)>,
}

impl<Context, Action, State, Output, F> SimpleLongRunningNode<Context, Action, State, Output, F>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    State: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Option<State>, &mut Context) -> Result<LongRunningOutcome<Output, State>, FloxideError>
        + Send
        + Sync
        + 'static,
{
    /// Create a new SimpleLongRunningNode with a unique ID
    pub fn new(process_fn: F) -> Self {
        Self {
            id: NodeId::new(),
            process_fn: Arc::new(process_fn),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a new SimpleLongRunningNode with a specified ID
    pub fn with_id(id: impl Into<NodeId>, process_fn: F) -> Self {
        Self {
            id: id.into(),
            process_fn: Arc::new(process_fn),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<Context, Action, State, Output, F> LongRunningNode<Context, Action>
    for SimpleLongRunningNode<Context, Action, State, Output, F>
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Debug,
    State: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
    Output: Send + Sync + 'static,
    F: Fn(Option<State>, &mut Context) -> Result<LongRunningOutcome<Output, State>, FloxideError>
        + Send
        + Sync
        + 'static,
{
    type State = State;
    type Output = Output;

    async fn process(
        &self,
        state: Option<Self::State>,
        ctx: &mut Context,
    ) -> Result<LongRunningOutcome<Self::Output, Self::State>, FloxideError> {
        (self.process_fn)(state, ctx)
    }

    fn id(&self) -> NodeId {
        self.id.clone()
    }
}

/// An adapter that allows a LongRunningNode to be used as a standard Node.
/// This adapter handles state management and action conversion.
pub struct LongRunningNodeAdapter<LRN, Context, Action, S>
where
    LRN: LongRunningNode<Context, Action>,
    Context: Send + Sync + 'static,
    Action: ActionType + LongRunningActionExt + Send + Sync + 'static + Debug,
    S: StateStore,
{
    node: Arc<LRN>,
    state_store: Arc<S>,
    suspend_timeout: Option<Duration>,
    _phantom: std::marker::PhantomData<(Context, Action)>,
}

impl<LRN, Context, Action, S> LongRunningNodeAdapter<LRN, Context, Action, S>
where
    LRN: LongRunningNode<Context, Action>,
    Context: Send + Sync + 'static,
    Action: ActionType + LongRunningActionExt + Send + Sync + 'static + Debug,
    S: StateStore,
{
    /// Create a new adapter for a long-running node
    pub fn new(node: LRN, state_store: S) -> Self {
        Self {
            node: Arc::new(node),
            state_store: Arc::new(state_store),
            suspend_timeout: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set a timeout for suspended nodes
    pub fn with_suspend_timeout(mut self, timeout: Duration) -> Self {
        self.suspend_timeout = Some(timeout);
        self
    }
}

#[async_trait]
impl<LRN, Context, Action, S> Node<Context, Action>
    for LongRunningNodeAdapter<LRN, Context, Action, S>
where
    LRN: LongRunningNode<Context, Action>,
    Context: Send + Sync + 'static,
    Action: ActionType + LongRunningActionExt + Send + Sync + 'static + Debug,
    S: StateStore,
    LRN::Output: Send + Sync + 'static, // Needed for Node trait bounds
{
    type Output = Action;

    async fn process(
        &self,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Self::Output, Action>, FloxideError> {
        // Get the node's state if it exists
        let node_id = self.node.id();
        let state = match self
            .state_store
            .get_state::<LRN::State>(node_id.clone())
            .await
        {
            Ok(state) => state,
            Err(e) => return Err(e),
        };

        // Process the next step
        match self.node.process(state, ctx).await {
            Ok(LongRunningOutcome::Complete(_output)) => {
                // Processing is complete, clean up state
                self.state_store.remove_state(node_id).await?;

                // Return with complete action
                Ok(NodeOutcome::RouteToAction(Action::complete()))
            }
            Ok(LongRunningOutcome::Suspend(state)) => {
                // Save the state
                self.state_store.save_state(node_id, &state).await?;

                // Return with suspend action
                Ok(NodeOutcome::RouteToAction(Action::suspend()))
            }
            Err(e) => Err(e),
        }
    }

    fn id(&self) -> NodeId {
        self.node.id()
    }
}

/// A workflow for orchestrating long-running nodes.
/// This manages the suspension and resumption of nodes.
pub struct LongRunningWorkflow<Context, Action, S>
where
    Context: Send + Sync + 'static,
    Action: ActionType + LongRunningActionExt + Send + Sync + 'static + Debug,
    S: StateStore,
{
    state_store: Arc<S>,
    _phantom: std::marker::PhantomData<(Context, Action)>,
}

impl<Context, Action, S> LongRunningWorkflow<Context, Action, S>
where
    Context: Send + Sync + 'static,
    Action: ActionType + LongRunningActionExt + Send + Sync + 'static + Debug,
    S: StateStore,
{
    /// Create a new long-running workflow
    pub fn new(state_store: S) -> Self {
        Self {
            state_store: Arc::new(state_store),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Execute a single long-running node until completion or suspension
    pub async fn execute_node<LRN>(
        &self,
        node: &LRN,
        ctx: &mut Context,
    ) -> Result<NodeOutcome<Action, Action>, FloxideError>
    where
        LRN: LongRunningNode<Context, Action>,
    {
        let node_id = node.id();
        let state = self
            .state_store
            .get_state::<LRN::State>(node_id.clone())
            .await?;

        match node.process(state, ctx).await? {
            LongRunningOutcome::Complete(_output) => {
                // Clean up state
                self.state_store.remove_state(node_id).await?;
                // Store output if needed

                // Return complete action
                Ok(NodeOutcome::Success(Action::complete()))
            }
            LongRunningOutcome::Suspend(state) => {
                // Save state
                self.state_store.save_state(node_id, &state).await?;

                // Return suspend action
                Ok(NodeOutcome::Success(Action::suspend()))
            }
        }
    }

    /// Check if a node has suspended state
    pub async fn has_suspended_state(&self, node_id: NodeId) -> Result<bool, FloxideError> {
        self.state_store.has_state(node_id).await
    }

    /// Get all node IDs with suspended state
    pub async fn get_suspended_nodes(&self) -> Result<Vec<NodeId>, FloxideError> {
        self.state_store.get_all_node_ids().await
    }
}

/// Trait for storing and retrieving node states.
#[async_trait]
pub trait StateStore: Send + Sync + 'static {
    /// Save a node's state
    async fn save_state<T: Serialize + Send + Sync + 'static>(
        &self,
        node_id: NodeId,
        state: &T,
    ) -> Result<(), FloxideError>;

    /// Get a node's state if it exists
    async fn get_state<T: for<'de> Deserialize<'de> + Send + Sync + 'static>(
        &self,
        node_id: NodeId,
    ) -> Result<Option<T>, FloxideError>;

    /// Check if a node has saved state
    async fn has_state(&self, node_id: NodeId) -> Result<bool, FloxideError>;

    /// Remove a node's state
    async fn remove_state(&self, node_id: NodeId) -> Result<(), FloxideError>;

    /// Get all node IDs with saved states
    async fn get_all_node_ids(&self) -> Result<Vec<NodeId>, FloxideError>;
}

/// A simple in-memory implementation of StateStore for testing.
pub struct InMemoryStateStore {
    states: Arc<Mutex<HashMap<NodeId, Vec<u8>>>>,
}

impl InMemoryStateStore {
    /// Create a new in-memory state store
    pub fn new() -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryStateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn save_state<T: Serialize + Send + Sync + 'static>(
        &self,
        node_id: NodeId,
        state: &T,
    ) -> Result<(), FloxideError> {
        let serialized = serde_json::to_vec(state)
            .map_err(|e| FloxideError::SerializationError(e.to_string()))?;

        let mut states = self
            .states
            .lock()
            .map_err(|e| FloxideError::Other(format!("Failed to acquire mutex lock: {}", e)))?;

        states.insert(node_id, serialized);
        Ok(())
    }

    async fn get_state<T: for<'de> Deserialize<'de> + Send + Sync + 'static>(
        &self,
        node_id: NodeId,
    ) -> Result<Option<T>, FloxideError> {
        let states = self
            .states
            .lock()
            .map_err(|e| FloxideError::Other(format!("Failed to acquire mutex lock: {}", e)))?;

        match states.get(&node_id) {
            Some(serialized) => {
                let state = serde_json::from_slice(serialized)
                    .map_err(|e| FloxideError::DeserializationError(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    async fn has_state(&self, node_id: NodeId) -> Result<bool, FloxideError> {
        let states = self
            .states
            .lock()
            .map_err(|e| FloxideError::Other(format!("Failed to acquire mutex lock: {}", e)))?;

        Ok(states.contains_key(&node_id))
    }

    async fn remove_state(&self, node_id: NodeId) -> Result<(), FloxideError> {
        let mut states = self
            .states
            .lock()
            .map_err(|e| FloxideError::Other(format!("Failed to acquire mutex lock: {}", e)))?;

        states.remove(&node_id);
        Ok(())
    }

    async fn get_all_node_ids(&self) -> Result<Vec<NodeId>, FloxideError> {
        let states = self
            .states
            .lock()
            .map_err(|e| FloxideError::Other(format!("Failed to acquire mutex lock: {}", e)))?;

        Ok(states.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_long_running_node() {
        // Create a node that completes after 3 steps
        let node = SimpleLongRunningNode::<_, DefaultAction, _, _, _>::new(
            |state: Option<u32>, _ctx: &mut ()| {
                let current_step = state.unwrap_or(0);
                let next_step = current_step + 1;

                if next_step >= 3 {
                    // Complete after 3 steps
                    Ok(LongRunningOutcome::Complete("done"))
                } else {
                    // Otherwise suspend with updated state
                    Ok(LongRunningOutcome::Suspend(next_step))
                }
            },
        );

        let state_store = InMemoryStateStore::new();
        let workflow = LongRunningWorkflow::new(state_store);

        let mut ctx = ();

        // First execution - should suspend with state 1
        let outcome = workflow.execute_node(&node, &mut ctx).await.unwrap();
        assert!(matches!(outcome, NodeOutcome::Success(action) if action.is_suspend()));

        // Verify state was saved
        assert!(workflow.has_suspended_state(node.id()).await.unwrap());

        // Second execution - should suspend with state 2
        let outcome = workflow.execute_node(&node, &mut ctx).await.unwrap();
        assert!(matches!(outcome, NodeOutcome::Success(action) if action.is_suspend()));

        // Third execution - should complete
        let outcome = workflow.execute_node(&node, &mut ctx).await.unwrap();
        assert!(matches!(outcome, NodeOutcome::Success(action) if action.is_complete()));

        // Verify state was removed
        assert!(!workflow.has_suspended_state(node.id()).await.unwrap());
    }
}
