use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::sync::Arc;

use tracing::{debug, error, info, warn};

use crate::action::{ActionType, DefaultAction};
use crate::error::FlowrsError;
use crate::node::{Node, NodeId, NodeOutcome};

/// Workflow execution error
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    /// Initial node not found
    #[error("Initial node not found: {0}")]
    InitialNodeNotFound(NodeId),

    /// Node not found
    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    /// Action not handled
    #[error("Action not handled: {0}")]
    ActionNotHandled(String),

    /// Node execution error
    #[error("Node execution error: {0}")]
    NodeExecution(#[from] FlowrsError),
}

/// A workflow composed of connected nodes
pub struct Workflow<Context, A = DefaultAction, Output = ()>
where
    A: ActionType,
{
    /// Initial node to start workflow execution from
    start_node: NodeId,

    /// All nodes in the workflow
    pub(crate) nodes: HashMap<NodeId, Arc<dyn Node<Context, A, Output = Output>>>,

    /// Connections between nodes and actions
    edges: HashMap<(NodeId, A), NodeId>,

    /// Default fallback routes for nodes
    default_routes: HashMap<NodeId, NodeId>,
}

impl<Context, A, Output> Workflow<Context, A, Output>
where
    Context: Send + Sync + 'static,
    A: ActionType + Debug + Default + Clone + Send + Sync + 'static,
    Output: Send + Sync + 'static + std::fmt::Debug,
{
    /// Create a new workflow with the given start node
    pub fn new<N>(start_node: N) -> Self
    where
        N: Node<Context, A, Output = Output> + 'static,
    {
        let id = start_node.id();
        let mut nodes = HashMap::new();
        nodes.insert(
            id.clone(),
            Arc::new(start_node) as Arc<dyn Node<Context, A, Output = Output>>,
        );

        Self {
            start_node: id,
            nodes,
            edges: HashMap::new(),
            default_routes: HashMap::new(),
        }
    }

    /// Add a node to the workflow
    pub fn add_node<N>(&mut self, node: N) -> &mut Self
    where
        N: Node<Context, A, Output = Output> + 'static,
    {
        let id = node.id();
        self.nodes.insert(
            id,
            Arc::new(node) as Arc<dyn Node<Context, A, Output = Output>>,
        );
        self
    }

    /// Connect a node to another node with an action
    pub fn connect(&mut self, from: &NodeId, action: A, to: &NodeId) -> &mut Self {
        self.edges.insert((from.clone(), action), to.clone());
        self
    }

    /// Set a default route from one node to another (used when no specific action matches)
    pub fn set_default_route(&mut self, from: &NodeId, to: &NodeId) -> &mut Self {
        self.default_routes.insert(from.clone(), to.clone());
        self
    }

    /// Get a node by its ID
    pub fn get_node(&self, id: NodeId) -> Option<&dyn Node<Context, A, Output = Output>> {
        self.nodes.get(&id).map(|node| node.as_ref())
    }

    /// Execute the workflow with the given context
    pub async fn execute(&self, ctx: &mut Context) -> Result<Output, WorkflowError> {
        let mut current_node_id = self.start_node.clone();
        let mut visited = HashSet::new();

        info!(start_node = %current_node_id, "Starting workflow execution");
        eprintln!("Starting workflow execution from node: {}", current_node_id);

        // Debug info for connections
        eprintln!("Node connections:");
        for ((from, action), to) in &self.edges {
            eprintln!("  {} -[{:?}]-> {}", from, action, to);
        }

        eprintln!("Default routes:");
        for (from, to) in &self.default_routes {
            eprintln!("  {} -> {}", from, to);
        }

        while !visited.contains(&current_node_id) {
            let node = self.nodes.get(&current_node_id).ok_or_else(|| {
                error!(node_id = %current_node_id, "Node not found in workflow");
                WorkflowError::NodeNotFound(current_node_id.clone())
            })?;

            visited.insert(current_node_id.clone());
            debug!(node_id = %current_node_id, "Executing node");

            let outcome = node
                .process(ctx)
                .await
                .map_err(WorkflowError::NodeExecution)?;

            match &outcome {
                NodeOutcome::Success(_) => {
                    info!(node_id = %current_node_id, "Node completed successfully with Success outcome");
                    eprintln!("Node {} completed with Success outcome", current_node_id);
                }
                NodeOutcome::Skipped => {
                    info!(node_id = %current_node_id, "Node completed with Skipped outcome");
                    eprintln!("Node {} completed with Skipped outcome", current_node_id);
                }
                NodeOutcome::RouteToAction(action) => {
                    info!(node_id = %current_node_id, action = %action.name(), "Node completed with RouteToAction outcome");
                    eprintln!(
                        "Node {} completed with RouteToAction({:?}) outcome",
                        current_node_id, action
                    );
                }
            }

            match outcome {
                NodeOutcome::Success(output) => {
                    info!(node_id = %current_node_id, "Node completed successfully");
                    // Find the default route if there is one
                    if let Some(next) = self.default_routes.get(&current_node_id) {
                        debug!(
                            node_id = %current_node_id,
                            next_node = %next,
                            "Following default route"
                        );
                        current_node_id = next.clone();
                    } else {
                        debug!(node_id = %current_node_id, "Workflow execution completed");
                        return Ok(output);
                    }
                }
                NodeOutcome::Skipped => {
                    warn!(node_id = %current_node_id, "Node was skipped");
                    // Find the default route if there is one
                    if let Some(next) = self.default_routes.get(&current_node_id) {
                        debug!(
                            node_id = %current_node_id,
                            next_node = %next,
                            "Following default route after skip"
                        );
                        current_node_id = next.clone();
                    } else {
                        warn!(node_id = %current_node_id, "Node was skipped but no default route exists");
                        return Err(WorkflowError::ActionNotHandled(
                            "Skipped node without default route".into(),
                        ));
                    }
                }
                NodeOutcome::RouteToAction(action) => {
                    debug!(
                        node_id = %current_node_id,
                        action = ?action,
                        "Node routed to action"
                    );

                    // Look for an edge matching this action
                    if let Some(next) = self.edges.get(&(current_node_id.clone(), action.clone())) {
                        debug!(
                            node_id = %current_node_id,
                            action = ?action,
                            next_node = %next,
                            "Following edge for action"
                        );
                        current_node_id = next.clone();
                    }
                    // If no matching edge, try the default action if this wasn't already the default action
                    else if action != A::default() {
                        if let Some(next) = self.edges.get(&(current_node_id.clone(), A::default()))
                        {
                            debug!(
                                node_id = %current_node_id,
                                next_node = %next,
                                "No edge for action, following default action"
                            );
                            current_node_id = next.clone();
                        } else if let Some(next) = self.default_routes.get(&current_node_id) {
                            debug!(
                                node_id = %current_node_id,
                                next_node = %next,
                                "No edge for action or default action, following default route"
                            );
                            current_node_id = next.clone();
                        } else {
                            error!(
                                node_id = %current_node_id,
                                action = ?action,
                                "No edge found for action and no default route"
                            );

                            // Debug info for connections
                            error!(
                                "Available edges: {:?}",
                                self.edges
                                    .iter()
                                    .map(|((from, act), to)| format!(
                                        "{} -[{:?}]-> {}",
                                        from, act, to
                                    ))
                                    .collect::<Vec<_>>()
                            );
                            error!(
                                "Default routes: {:?}",
                                self.default_routes
                                    .iter()
                                    .map(|(from, to)| format!("{} -> {}", from, to))
                                    .collect::<Vec<_>>()
                            );

                            return Err(WorkflowError::ActionNotHandled(format!("{:?}", action)));
                        }
                    } else if let Some(next) = self.default_routes.get(&current_node_id) {
                        debug!(
                            node_id = %current_node_id,
                            next_node = %next,
                            "No edge for default action, following default route"
                        );
                        current_node_id = next.clone();
                    } else {
                        error!(
                            node_id = %current_node_id,
                            action = ?action,
                            "No edge found for default action and no default route"
                        );

                        // Debug info for connections
                        error!(
                            "Available edges: {:?}",
                            self.edges
                                .iter()
                                .map(|((from, act), to)| format!("{} -[{:?}]-> {}", from, act, to))
                                .collect::<Vec<_>>()
                        );
                        error!(
                            "Default routes: {:?}",
                            self.default_routes
                                .iter()
                                .map(|(from, to)| format!("{} -> {}", from, to))
                                .collect::<Vec<_>>()
                        );

                        return Err(WorkflowError::ActionNotHandled(
                            "Default action not handled".into(),
                        ));
                    }
                }
            }
        }

        // If we get here, we've detected a cycle
        error!(
            node_id = %current_node_id,
            "Cycle detected in workflow execution"
        );
        Err(WorkflowError::NodeExecution(
            FlowrsError::WorkflowCycleDetected,
        ))
    }
}

// Implement Clone for Workflow
impl<Context, A, Output> Clone for Workflow<Context, A, Output>
where
    Context: Send + Sync + 'static,
    A: ActionType + Clone + Send + Sync + 'static,
    Output: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            start_node: self.start_node.clone(),
            nodes: self.nodes.clone(), // Cloning HashMap<NodeId, Arc<dyn Node>> is now possible
            edges: self.edges.clone(),
            default_routes: self.default_routes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::node;

    #[derive(Debug, Clone)]
    struct TestContext {
        value: i32,
        visited: Vec<String>,
    }

    #[tokio::test]
    async fn test_simple_linear_workflow() {
        // Create nodes
        let start_node = node::node(|mut ctx: TestContext| async move {
            ctx.value += 1;
            ctx.visited.push("start".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let middle_node = node::node(|mut ctx: TestContext| async move {
            ctx.value *= 2;
            ctx.visited.push("middle".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let end_node = node::node(|mut ctx: TestContext| async move {
            ctx.value -= 3;
            ctx.visited.push("end".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        // Build workflow
        let mut workflow = Workflow::new(start_node);
        let start_id = workflow.start_node.clone();
        let middle_id = middle_node.id();
        let end_id = end_node.id();

        workflow
            .add_node(middle_node)
            .add_node(end_node)
            .set_default_route(&start_id, &middle_id)
            .set_default_route(&middle_id, &end_id);

        // Execute workflow
        let mut ctx = TestContext {
            value: 10,
            visited: vec![],
        };

        let result = workflow.execute(&mut ctx).await;
        assert!(result.is_ok());

        // Check final state
        assert_eq!(ctx.value, 19); // 10 + 1 = 11 -> 11 * 2 = 22 -> 22 - 3 = 19
        assert_eq!(ctx.visited, vec!["start", "middle", "end"]);
    }

    #[tokio::test]
    async fn test_workflow_with_routing() {
        // Define custom action
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        enum TestAction {
            Default,
            Route1,
            Route2,
        }

        impl Default for TestAction {
            fn default() -> Self {
                Self::Default
            }
        }

        impl ActionType for TestAction {
            fn name(&self) -> &str {
                match self {
                    Self::Default => "default",
                    Self::Route1 => "route1",
                    Self::Route2 => "route2",
                }
            }
        }

        // Create nodes
        let start_node = node::node(|mut ctx: TestContext| async move {
            ctx.visited.push("start".to_string());
            // Route based on value
            if ctx.value > 5 {
                Ok((
                    ctx,
                    NodeOutcome::<(), TestAction>::RouteToAction(TestAction::Route1),
                ))
            } else {
                Ok((
                    ctx,
                    NodeOutcome::<(), TestAction>::RouteToAction(TestAction::Route2),
                ))
            }
        });

        let path1_node = node::node(|mut ctx: TestContext| async move {
            ctx.value += 100;
            ctx.visited.push("path1".to_string());
            Ok((ctx, NodeOutcome::<(), TestAction>::Success(())))
        });

        let path2_node = node::node(|mut ctx: TestContext| async move {
            ctx.value *= 10;
            ctx.visited.push("path2".to_string());
            Ok((ctx, NodeOutcome::<(), TestAction>::Success(())))
        });

        // Build workflow
        let mut workflow = Workflow::<_, TestAction, _>::new(start_node);
        let start_id = workflow.start_node.clone();
        let path1_id = path1_node.id();
        let path2_id = path2_node.id();

        workflow
            .add_node(path1_node)
            .add_node(path2_node)
            .connect(&start_id, TestAction::Route1, &path1_id)
            .connect(&start_id, TestAction::Route2, &path2_id);

        // Execute workflow - should take path 1
        let mut ctx1 = TestContext {
            value: 10,
            visited: vec![],
        };

        let result1 = workflow.execute(&mut ctx1).await;
        assert!(result1.is_ok());
        assert_eq!(ctx1.value, 110); // 10 -> route1 -> +100 = 110
        assert_eq!(ctx1.visited, vec!["start", "path1"]);

        // Execute workflow - should take path 2
        let mut ctx2 = TestContext {
            value: 3,
            visited: vec![],
        };

        let result2 = workflow.execute(&mut ctx2).await;
        assert!(result2.is_ok());
        assert_eq!(ctx2.value, 30); // 3 -> route2 -> *10 = 30
        assert_eq!(ctx2.visited, vec!["start", "path2"]);
    }

    #[tokio::test]
    async fn test_workflow_with_skipped_node() {
        let start_node = node::node(|mut ctx: TestContext| async move {
            ctx.visited.push("start".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let skip_node = node::node(|mut ctx: TestContext| async move {
            ctx.visited.push("skip_check".to_string());
            if ctx.value > 5 {
                // Skip this node
                Ok((ctx, NodeOutcome::<(), DefaultAction>::Skipped))
            } else {
                ctx.value *= 2;
                Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
            }
        });

        let end_node = node::node(|mut ctx: TestContext| async move {
            ctx.visited.push("end".to_string());
            ctx.value += 5;
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        // Build workflow
        let mut workflow = Workflow::new(start_node);
        let start_id = workflow.start_node.clone();
        let skip_id = skip_node.id();
        let end_id = end_node.id();

        workflow
            .add_node(skip_node)
            .add_node(end_node)
            .set_default_route(&start_id, &skip_id)
            .set_default_route(&skip_id, &end_id);

        // Execute workflow - should skip the middle node
        let mut ctx1 = TestContext {
            value: 10,
            visited: vec![],
        };

        let result1 = workflow.execute(&mut ctx1).await;
        assert!(result1.is_ok());
        assert_eq!(ctx1.value, 15); // 10 -> skip middle -> +5 = 15
        assert_eq!(ctx1.visited, vec!["start", "skip_check", "end"]);

        // Execute workflow - should not skip the middle node
        let mut ctx2 = TestContext {
            value: 3,
            visited: vec![],
        };

        let result2 = workflow.execute(&mut ctx2).await;
        assert!(result2.is_ok());
        assert_eq!(ctx2.value, 11); // 3 -> *2 = 6 -> +5 = 11
        assert_eq!(ctx2.visited, vec!["start", "skip_check", "end"]);
    }
}
