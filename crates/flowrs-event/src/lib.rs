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
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// A node that waits for events and processes them as they arrive
#[async_trait]
pub trait EventDrivenNode<Event, Context, Action>: Send + Sync
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    /// Wait for an external event to occur
    async fn wait_for_event(&self) -> Result<Event, FlowrsError>;
    
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
    /// Create a new channel-based event source
    ///
    /// Returns the event source and a sender that can be used to send events to it
    pub fn new(capacity: usize) -> (Self, mpsc::Sender<Event>) {
        let (sender, receiver) = mpsc::channel(capacity);
        let id = Uuid::new_v4().to_string();
        (Self { receiver, id }, sender)
    }
    
    /// Create a new channel-based event source with a specific ID
    pub fn with_id(capacity: usize, id: impl Into<String>) -> (Self, mpsc::Sender<Event>) {
        let (sender, receiver) = mpsc::channel(capacity);
        (Self { receiver, id: id.into() }, sender)
    }
}

#[async_trait]
impl<Event, Context, Action> EventDrivenNode<Event, Context, Action> for ChannelEventSource<Event>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    async fn wait_for_event(&self) -> Result<Event, FlowrsError> {
        self.receiver.recv().await.ok_or_else(|| FlowrsError::node_execution(
            self.id(), 
            "Event channel closed"
        ))
    }
    
    async fn process_event(
        &self, 
        event: Event, 
        _ctx: &mut Context
    ) -> Result<Action, FlowrsError> {
        // Default implementation just forwards the event
        debug!(node_id = %self.id(), "Event received but no custom processor defined");
        Ok(Action::default())
    }
    
    fn id(&self) -> NodeId {
        self.id.clone()
    }
}

/// A wrapper for an event processor that handles events from a channel
pub struct EventProcessor<Event, Context, Action, F>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    F: Fn(Event, &mut Context) -> Result<Action, FlowrsError> + Send + Sync + 'static,
{
    source: ChannelEventSource<Event>,
    processor: F,
}

impl<Event, Context, Action, F> EventProcessor<Event, Context, Action, F>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    F: Fn(Event, &mut Context) -> Result<Action, FlowrsError> + Send + Sync + 'static,
{
    /// Create a new event processor with a channel source
    pub fn new(capacity: usize, processor: F) -> (Self, mpsc::Sender<Event>) {
        let (source, sender) = ChannelEventSource::new(capacity);
        (Self { source, processor }, sender)
    }
    
    /// Create a new event processor with a specific ID
    pub fn with_id(capacity: usize, id: impl Into<String>, processor: F) -> (Self, mpsc::Sender<Event>) {
        let (source, sender) = ChannelEventSource::with_id(capacity, id);
        (Self { source, processor }, sender)
    }
}

#[async_trait]
impl<Event, Context, Action, F> EventDrivenNode<Event, Context, Action> for EventProcessor<Event, Context, Action, F>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    F: Fn(Event, &mut Context) -> Result<Action, FlowrsError> + Send + Sync + 'static,
{
    async fn wait_for_event(&self) -> Result<Event, FlowrsError> {
        self.source.wait_for_event().await
    }
    
    async fn process_event(
        &self, 
        event: Event, 
        ctx: &mut Context
    ) -> Result<Action, FlowrsError> {
        (self.processor)(event, ctx)
    }
    
    fn id(&self) -> NodeId {
        self.source.id()
    }
}

/// A workflow that processes events using event-driven nodes
pub struct EventDrivenWorkflow<Event, Context, Action>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
{
    nodes: HashMap<NodeId, Arc<dyn EventDrivenNode<Event, Context, Action>>>,
    routes: HashMap<(NodeId, Action), NodeId>,
    initial_node: NodeId,
    termination_action: Action,
}

impl<Event, Context, Action> EventDrivenWorkflow<Event, Context, Action>
where
    Event: Send + 'static,
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    Action: PartialEq + Eq + std::hash::Hash + Clone,
{
    /// Create a new event-driven workflow with an initial node
    pub fn new(
        initial_node: Arc<dyn EventDrivenNode<Event, Context, Action>>,
        termination_action: Action,
    ) -> Self {
        let initial_id = initial_node.id();
        let mut nodes = HashMap::new();
        nodes.insert(initial_id.clone(), initial_node);
        
        Self {
            nodes,
            routes: HashMap::new(),
            initial_node: initial_id,
            termination_action,
        }
    }
    
    /// Add a node to the workflow
    pub fn add_node(&mut self, node: Arc<dyn EventDrivenNode<Event, Context, Action>>) {
        let id = node.id();
        debug!(node_id = %id, "Adding event-driven node to workflow");
        self.nodes.insert(id, node);
    }
    
    /// Set a route from one node to another based on an action
    pub fn set_route(&mut self, from_id: &NodeId, action: Action, to_id: &NodeId) {
        debug!(from = %from_id, to = %to_id, action = %action.name(), "Setting route in event-driven workflow");
        self.routes.insert((from_id.clone(), action), to_id.clone());
    }
    
    /// Execute the workflow, processing events until completion or error
    pub async fn execute(&self, ctx: &mut Context) -> Result<(), FlowrsError> {
        let mut current_node_id = self.initial_node.clone();
        
        loop {
            let node = self.nodes.get(&current_node_id).ok_or_else(|| {
                FlowrsError::node_not_found(current_node_id.clone())
            })?;
            
            info!(node_id = %current_node_id, "Waiting for event in node");
            
            // Wait for an event
            let event = node.wait_for_event().await?;
            
            debug!(node_id = %current_node_id, "Processing event in node");
            
            // Process the event
            let action = node.process_event(event, ctx).await?;
            
            debug!(node_id = %current_node_id, action = %action.name(), "Event processed with action");
            
            // Check for termination action
            if action == self.termination_action {
                info!(action = %action.name(), "Termination action received, ending workflow");
                break;
            }
            
            // Route to the next node
            if let Some(next_node_id) = self.routes.get(&(current_node_id.clone(), action.clone())) {
                debug!(from = %current_node_id, to = %next_node_id, action = %action.name(), "Routing to next node");
                current_node_id = next_node_id.clone();
            } else {
                debug!(node_id = %current_node_id, action = %action.name(), "No route defined for action, staying on same node");
                // Stay on the same node if no route is defined for this action
            }
        }
        
        Ok(())
    }
    
    /// Execute the workflow with a timeout
    pub async fn execute_with_timeout(
        &self, 
        ctx: &mut Context, 
        timeout: Duration
    ) -> Result<(), FlowrsError> {
        tokio::select! {
            result = self.execute(ctx) => result,
            _ = tokio::time::sleep(timeout) => {
                warn!("Event-driven workflow execution timed out after {:?}", timeout);
                Err(FlowrsError::timeout("Event-driven workflow execution timed out"))
            }
        }
    }
}

/// An adapter to integrate event-driven nodes with standard workflows
pub struct EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    node: Arc<dyn EventDrivenNode<E, C, A>>,
    id: NodeId,
    timeout: Duration,
    timeout_action: A,
}

impl<E, C, A> EventDrivenNodeAdapter<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
{
    /// Create a new adapter for an event-driven node
    pub fn new(
        node: Arc<dyn EventDrivenNode<E, C, A>>,
        timeout: Duration,
        timeout_action: A,
    ) -> Self {
        Self {
            id: node.id(),
            node,
            timeout,
            timeout_action,
        }
    }
    
    /// Create a new adapter with a specific ID
    pub fn with_id(
        node: Arc<dyn EventDrivenNode<E, C, A>>,
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
    A: ActionType + Send + Sync + 'static + Clone,
{
    type Output = ();
    
    fn id(&self) -> NodeId {
        self.id.clone()
    }
    
    async fn process(&self, ctx: &mut C) -> Result<NodeOutcome<Self::Output, A>, FlowrsError> {
        debug!(node_id = %self.id, "Processing event-driven node within standard workflow");
        
        // Wait for one event with timeout
        tokio::select! {
            result = self.node.wait_for_event() => {
                match result {
                    Ok(event) => {
                        let action = self.node.process_event(event, ctx).await?;
                        debug!(node_id = %self.id, action = %action.name(), "Event processed in standard workflow");
                        Ok(NodeOutcome::RouteToAction((), action))
                    },
                    Err(e) => Err(e),
                }
            }
            _ = tokio::time::sleep(self.timeout) => {
                warn!(node_id = %self.id, timeout = ?self.timeout, "Event-driven node timed out waiting for event");
                Ok(NodeOutcome::RouteToAction((), self.timeout_action.clone()))
            }
        }
    }
}

/// An adapter to integrate an entire event-driven workflow within a standard workflow
pub struct NestedEventDrivenWorkflow<E, C, A>
where
    E: Send + 'static,
    C: Send + Sync + 'static,
    A: ActionType + Send + Sync + 'static,
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
    A: ActionType + Send + Sync + 'static,
{
    /// Create a new nested event-driven workflow
    pub fn new(
        workflow: Arc<EventDrivenWorkflow<E, C, A>>,
        complete_action: A,
        timeout_action: A,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            workflow,
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
            id: Uuid::new_v4().to_string(),
            workflow,
            timeout: Some(timeout),
            complete_action,
            timeout_action,
        }
    }
    
    /// Create a nested workflow with a specific ID
    pub fn with_id(
        workflow: Arc<EventDrivenWorkflow<E, C, A>>,
        id: impl Into<String>,
        complete_action: A,
        timeout_action: A,
    ) -> Self {
        Self {
            id: id.into(),
            workflow,
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
    A: ActionType + Send + Sync + 'static + Clone,
{
    type Output = ();
    
    fn id(&self) -> NodeId {
        self.id.clone()
    }
    
    async fn process(&self, ctx: &mut C) -> Result<NodeOutcome<Self::Output, A>, FlowrsError> {
        info!(node_id = %self.id, "Executing nested event-driven workflow");
        
        // Execute the workflow, optionally with a timeout
        let result = if let Some(timeout) = self.timeout {
            self.workflow.execute_with_timeout(ctx, timeout).await
        } else {
            self.workflow.execute(ctx).await
        };
        
        match result {
            Ok(()) => {
                info!(node_id = %self.id, "Nested event-driven workflow completed successfully");
                Ok(NodeOutcome::RouteToAction((), self.complete_action.clone()))
            },
            Err(e) => {
                if e.is_timeout() {
                    warn!(node_id = %self.id, "Nested event-driven workflow timed out");
                    Ok(NodeOutcome::RouteToAction((), self.timeout_action.clone()))
                } else {
                    Err(e)
                }
            }
        }
    }
}

/// Extension trait for `ActionType` to provide common event-driven workflow actions
pub trait EventActionExt: ActionType {
    /// Create a terminate action for event-driven workflows
    fn terminate() -> Self;
    
    /// Create a timeout action for timed operations
    fn timeout() -> Self;
}

impl EventActionExt for DefaultAction {
    fn terminate() -> Self {
        DefaultAction::Terminate
    }
    
    fn timeout() -> Self {
        DefaultAction::Timeout
    }
} 