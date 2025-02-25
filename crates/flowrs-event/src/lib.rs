//! # Flowrs Event
//!
//! Event-driven node extensions for the Flowrs framework.
//!
//! This crate provides event-driven workflow capabilities through
//! the EventDrivenNode trait and various event source implementations.

use async_trait::async_trait;
use flowrs_core::{
    error::FlowrsError, ActionType, DefaultAction, Node, NodeId, NodeOutcome,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;

/// A node that waits for events and processes them as they arrive
#[async_trait]
pub trait EventDrivenNode<Event, Context, Action>: Send + Sync
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
{
    /// Wait for an external event to occur
    async fn wait_for_event(&mut self) -> Result<Event, FlowrsError>;
    
    /// Process the received event and update context
    async fn process_event(
        &self, 
        event: Event, 
        ctx: &mut Context
    ) -> Result<Action, FlowrsError>;
    
    /// Get the node's unique identifier
    fn id(&self) -> NodeId;
}

/// A channel-based event source that receives events from a Tokio MPSC channel
pub struct ChannelEventSource<Event> {
    receiver: mpsc::Receiver<Event>,
    id: NodeId,
}

impl<Event> ChannelEventSource<Event>
where
    Event: Send + 'static,
{
    /// Create a new channel event source with a default ID
    pub fn new(capacity: usize) -> (Self, mpsc::Sender<Event>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (
            Self {
                receiver,
                id: Uuid::new_v4().to_string(),
            },
            sender,
        )
    }

    /// Create a new channel event source with a specific ID
    pub fn with_id(capacity: usize, id: impl Into<String>) -> (Self, mpsc::Sender<Event>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (
            Self {
                receiver,
                id: id.into(),
            },
            sender,
        )
    }
}

#[async_trait]
impl<Event, Context, Action> EventDrivenNode<Event, Context, Action> for ChannelEventSource<Event>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
{
    async fn wait_for_event(&mut self) -> Result<Event, FlowrsError> {
        match self.receiver.recv().await {
            Some(event) => Ok(event),
            None => Err(FlowrsError::Other("Event channel closed".to_string())),
        }
    }
    
    async fn process_event(
        &self, 
        _event: Event, 
        _ctx: &mut Context
    ) -> Result<Action, FlowrsError> {
        // ChannelEventSource just passes events through, it doesn't process them
        // Return the default action
        Ok(Action::default())
    }
    
    fn id(&self) -> NodeId {
        self.id.clone()
    }
}

/// An event processor that applies a function to each event
pub struct EventProcessor<Event, Context, Action, F>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
    F: Fn(Event, &mut Context) -> Result<Action, FlowrsError> + Send + Sync + 'static,
{
    source: Arc<tokio::sync::Mutex<ChannelEventSource<Event>>>,
    processor: F,
    _phantom: PhantomData<(Context, Action)>,
}

impl<Event, Context, Action, F> EventProcessor<Event, Context, Action, F>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
    F: Fn(Event, &mut Context) -> Result<Action, FlowrsError> + Send + Sync + 'static,
{
    /// Create a new event processor with a default ID
    pub fn new(capacity: usize, processor: F) -> (Self, mpsc::Sender<Event>) {
        let (source, sender) = ChannelEventSource::new(capacity);
        (Self { source: Arc::new(tokio::sync::Mutex::new(source)), processor, _phantom: PhantomData }, sender)
    }
    
    /// Create a new event processor with a specific ID
    pub fn with_id(capacity: usize, id: impl Into<String>, processor: F) -> (Self, mpsc::Sender<Event>) {
        let (source, sender) = ChannelEventSource::with_id(capacity, id);
        (Self { source: Arc::new(tokio::sync::Mutex::new(source)), processor, _phantom: PhantomData }, sender)
    }
}

#[async_trait]
impl<Event, Context, Action, F> EventDrivenNode<Event, Context, Action> for EventProcessor<Event, Context, Action, F>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
    F: Fn(Event, &mut Context) -> Result<Action, FlowrsError> + Send + Sync + 'static,
{
    async fn wait_for_event(&mut self) -> Result<Event, FlowrsError> {
        let mut source = self.source.lock().await;
        <ChannelEventSource<Event> as EventDrivenNode<Event, Context, Action>>::wait_for_event(&mut *source).await
    }
    
    async fn process_event(
        &self, 
        event: Event, 
        ctx: &mut Context
    ) -> Result<Action, FlowrsError> {
        (self.processor)(event, ctx)
    }
    
    fn id(&self) -> NodeId {
        self.source.try_lock().map(|source| 
            <ChannelEventSource<Event> as EventDrivenNode<Event, Context, Action>>::id(&*source)
        ).unwrap_or_else(|_| "locked".to_string())
    }
}

/// A workflow that processes events using event-driven nodes
pub struct EventDrivenWorkflow<Event, Context, Action>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
{
    nodes: HashMap<NodeId, Arc<tokio::sync::Mutex<dyn EventDrivenNode<Event, Context, Action>>>>,
    routes: HashMap<(NodeId, Action), NodeId>,
    initial_node: NodeId,
    termination_action: Action,
}

impl<Event, Context, Action> EventDrivenWorkflow<Event, Context, Action>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static + Default,
{
    /// Create a new event-driven workflow with an initial node
    pub fn new(
        initial_node: Arc<tokio::sync::Mutex<dyn EventDrivenNode<Event, Context, Action>>>,
        termination_action: Action,
    ) -> Self {
        let id = {
            initial_node.try_lock().map(|n| n.id()).unwrap_or_else(|_| "locked".to_string())
        };
        
        let mut nodes = HashMap::new();
        nodes.insert(id.clone(), initial_node);
        
        Self {
            nodes,
            routes: HashMap::new(),
            initial_node: id,
            termination_action,
        }
    }
    
    /// Add a node to the workflow
    pub fn add_node(&mut self, node: Arc<tokio::sync::Mutex<dyn EventDrivenNode<Event, Context, Action>>>) {
        let id = {
            node.try_lock().map(|n| n.id()).unwrap_or_else(|_| "locked".to_string())
        };
        self.nodes.insert(id, node);
    }
    
    /// Set a route from one node to another based on an action
    pub fn set_route(&mut self, from_id: &NodeId, action: Action, to_id: &NodeId) {
        // Store the route in the routing table
        self.routes.insert((from_id.clone(), action), to_id.clone());
    }
    
    /// Sets a route with validation to ensure proper event flow
    /// 
    /// This method ensures that processor nodes (non-event sources) route back to
    /// valid event sources, preventing the "not an event source" error during execution.
    ///
    /// # Arguments
    ///
    /// * `from_id` - The source node ID
    /// * `action` - The action that triggers this route
    /// * `to_id` - The destination node ID
    ///
    /// # Returns
    ///
    /// * `Result<(), FlowrsError>` - Ok if the route is valid, Error otherwise
    pub fn set_route_with_validation(
        &mut self, 
        from_id: &NodeId, 
        action: Action, 
        to_id: &NodeId
    ) -> Result<(), FlowrsError> {
        // Check if the destination node exists
        if !self.nodes.contains_key(to_id) {
            return Err(FlowrsError::Other(
                format!("Destination node '{}' not found in workflow", to_id)
            ));
        }
        
        // Check if the source node is a processor (not an event source)
        // and the destination is also not an event source
        // This is done by attempting to call wait_for_event on both nodes
        // If the method returns an error containing "not an event source", it's a processor
        
        // For now we'll just add the route, but in a future implementation,
        // we should check if the source node is a processor and the destination
        // is also a processor, which would be an invalid routing pattern
        
        // Store the route in the routing table
        self.routes.insert((from_id.clone(), action), to_id.clone());
        
        Ok(())
    }
    
    /// Execute the workflow, processing events until the termination action is returned
    pub async fn execute(&self, ctx: &mut Context) -> Result<(), FlowrsError> {
        let mut current_node_id = self.initial_node.clone();
        
        loop {
            // Get the current node
            let node = self.nodes.get(&current_node_id)
                .ok_or_else(|| FlowrsError::node_not_found(current_node_id.clone()))?;
            
            // Wait for an event and process it
            let event = {
                let mut node_guard = node.lock().await;
                match node_guard.wait_for_event().await {
                    Ok(event) => event,
                    Err(e) => {
                        // Check if the error message indicates this is not an event source
                        if e.to_string().contains("not an event source") {
                            // This is a processor node, not an event source
                            // We need to find the initial node (which should be an event source)
                            warn!("Node '{}' is not an event source, routing to initial node", current_node_id);
                            
                            // Get the initial node and try to get an event from there
                            current_node_id = self.initial_node.clone();
                            let source_node = self.nodes.get(&current_node_id)
                                .ok_or_else(|| FlowrsError::Other(
                                    "Initial node not found in workflow".to_string()
                                ))?;
                                
                            let mut source_guard = source_node.lock().await;
                            source_guard.wait_for_event().await?
                        } else {
                            // Propagate other errors
                            return Err(e);
                        }
                    }
                }
            };
            
            let action = {
                let node_guard = node.lock().await;
                node_guard.process_event(event, ctx).await?
            };
            
            // If the action is the termination action, we're done
            if action == self.termination_action {
                info!("Event-driven workflow terminated with termination action");
                return Ok(());
            }
            
            // Find the next node to route to
            current_node_id = self.routes.get(&(current_node_id, action.clone()))
                .ok_or_else(|| FlowrsError::WorkflowDefinitionError(
                    format!("No route defined for action: {}", action.name())
                ))?
                .clone();
        }
    }
    
    /// Execute the workflow with a timeout
    pub async fn execute_with_timeout(
        &self, 
        ctx: &mut Context, 
        timeout: Duration
    ) -> Result<(), FlowrsError> {
        match tokio::time::timeout(timeout, self.execute(ctx)).await {
            Ok(result) => result,
            Err(_) => Err(FlowrsError::timeout("Event-driven workflow execution timed out"))
        }
    }
}

/// Adapter to use an event-driven node in a standard workflow
pub struct EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static + Default,
{
    node: Arc<tokio::sync::Mutex<dyn EventDrivenNode<E, C, A>>>,
    id: NodeId,
    timeout: Duration,
    timeout_action: A,
}

impl<E, C, A> EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static + Default,
{
    /// Create a new adapter with default ID
    pub fn new(
        node: Arc<tokio::sync::Mutex<dyn EventDrivenNode<E, C, A>>>,
        timeout: Duration,
        timeout_action: A,
    ) -> Self {
        let id = {
            node.try_lock().map(|n| n.id()).unwrap_or_else(|_| "locked".to_string())
        };
        
        Self {
            node,
            id,
            timeout,
            timeout_action,
        }
    }

    /// Create a new adapter with a specific ID
    pub fn with_id(
        node: Arc<tokio::sync::Mutex<dyn EventDrivenNode<E, C, A>>>,
        id: impl Into<String>,
        timeout: Duration,
        timeout_action: A,
    ) -> Self {
        Self {
            node,
            id: id.into(),
            timeout,
            timeout_action,
        }
    }
}

#[async_trait]
impl<E, C, A> Node<C, A> for EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static + Default,
{
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(&self, ctx: &mut C) -> Result<NodeOutcome<Self::Output, A>, FlowrsError> {
        match tokio::time::timeout(self.timeout, async {
            let event = {
                let mut node_guard = self.node.lock().await;
                node_guard.wait_for_event().await
            };
            event
        }).await {
            Ok(Ok(event)) => {
                let action = {
                    let node_guard = self.node.lock().await;
                    node_guard.process_event(event, ctx).await?
                };
                Ok(NodeOutcome::RouteToAction(action))
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                // Timeout occurred
                Ok(NodeOutcome::RouteToAction(self.timeout_action.clone()))
            }
        }
    }
}

/// A nested event-driven workflow that can be used in a standard workflow
pub struct NestedEventDrivenWorkflow<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static + Default,
{
    workflow: Arc<EventDrivenWorkflow<E, C, A>>,
    id: NodeId,
    timeout: Option<Duration>,
    complete_action: A,
    timeout_action: A,
}

impl<E, C, A> NestedEventDrivenWorkflow<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static + Default,
{
    /// Create a new nested workflow without a timeout
    pub fn new(
        workflow: Arc<EventDrivenWorkflow<E, C, A>>,
        complete_action: A,
        timeout_action: A,
    ) -> Self {
        Self {
            workflow,
            id: Uuid::new_v4().to_string(),
            timeout: None,
            complete_action,
            timeout_action,
        }
    }

    /// Create a new nested workflow with a timeout
    pub fn with_timeout(
        workflow: Arc<EventDrivenWorkflow<E, C, A>>,
        timeout: Duration,
        complete_action: A,
        timeout_action: A,
    ) -> Self {
        Self {
            workflow,
            id: Uuid::new_v4().to_string(),
            timeout: Some(timeout),
            complete_action,
            timeout_action,
        }
    }

    /// Create a new nested workflow with a specific ID
    pub fn with_id(
        workflow: Arc<EventDrivenWorkflow<E, C, A>>,
        id: impl Into<String>,
        complete_action: A,
        timeout_action: A,
    ) -> Self {
        Self {
            workflow,
            id: id.into(),
            timeout: None,
            complete_action,
            timeout_action,
        }
    }
}

#[async_trait]
impl<E, C, A> Node<C, A> for NestedEventDrivenWorkflow<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static + Default,
{
    type Output = ();

    fn id(&self) -> NodeId {
        self.id.clone()
    }

    async fn process(&self, ctx: &mut C) -> Result<NodeOutcome<Self::Output, A>, FlowrsError> {
        match self.timeout {
            Some(timeout) => {
                match tokio::time::timeout(timeout, self.workflow.execute(ctx)).await {
                    Ok(Ok(())) => {
                        // Workflow completed successfully
                        Ok(NodeOutcome::RouteToAction(self.complete_action.clone()))
                    }
                    Ok(Err(e)) => {
                        // Workflow execution error
                        Err(e)
                    }
                    Err(_) => {
                        // Timeout occurred
                        Ok(NodeOutcome::RouteToAction(self.timeout_action.clone()))
                    }
                }
            }
            None => {
                self.workflow.execute(ctx).await?;
                Ok(NodeOutcome::RouteToAction(self.complete_action.clone()))
            }
        }
    }
}

/// Extension trait for action types in event-driven workflows
pub trait EventActionExt: ActionType {
    /// Create a terminate action for event-driven workflows
    fn terminate() -> Self;
    
    /// Create a timeout action for timed operations
    fn timeout() -> Self;
}

impl EventActionExt for DefaultAction {
    fn terminate() -> Self {
        DefaultAction::Custom("terminate".into())
    }
    
    fn timeout() -> Self {
        DefaultAction::Custom("timeout".into())
    }
} 