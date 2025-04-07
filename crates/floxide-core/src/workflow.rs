use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use tracing::{debug, error, warn};

use crate::action::{ActionType, DefaultAction};
use crate::error::FloxideError;
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
    NodeExecution(#[from] FloxideError),
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

    /// Whether to allow cycles in the workflow
    allow_cycles: bool,

    /// Maximum number of times a node can be visited (0 means no limit)
    cycle_limit: usize,
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
            allow_cycles: false,
            cycle_limit: 0,
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

    /// Configure whether to allow cycles in the workflow
    ///
    /// By default, cycles are not allowed and will result in a WorkflowCycleDetected error.
    /// When cycles are allowed, the workflow will continue execution even when revisiting nodes.
    ///
    /// # Arguments
    /// * `allow` - Whether to allow cycles in the workflow
    pub fn allow_cycles(&mut self, allow: bool) -> &mut Self {
        self.allow_cycles = allow;
        self
    }

    /// Set a limit on the number of times a node can be visited
    ///
    /// This is only relevant when cycles are allowed. If a node is visited more than
    /// the specified limit, a WorkflowCycleDetected error will be returned.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of times a node can be visited (0 means no limit)
    pub fn set_cycle_limit(&mut self, limit: usize) -> &mut Self {
        self.cycle_limit = limit;
        self
    }

    /// Execute the workflow with the given context
    pub async fn execute(&self, ctx: &mut Context) -> Result<Output, WorkflowError> {
        let mut current_node_id = self.start_node.clone();
        let mut visit_counts = HashMap::new();

        debug!(start_node = %current_node_id, "Starting workflow execution");
        debug!(node = %current_node_id, "Starting workflow execution from node");

        // Debug info for connections
        debug!("Node connections:");
        for ((from, action), to) in &self.edges {
            debug!(from = %from, action = ?action, to = %to, "Connection");
        }

        debug!("Default routes:");
        for (from, to) in &self.default_routes {
            debug!(from = %from, to = %to, "Default route");
        }

        loop {
            // Check if we've visited this node before
            let visit_count = visit_counts.entry(current_node_id.clone()).or_insert(0);
            *visit_count += 1;

            // Check for cycles
            if !self.allow_cycles && *visit_count > 1 {
                // Cycles are not allowed and we've already visited this node
                error!(
                    node_id = %current_node_id,
                    "Cycle detected in workflow execution"
                );
                return Err(WorkflowError::NodeExecution(
                    FloxideError::WorkflowCycleDetected,
                ));
            }

            // Check cycle limit if specified (0 means no limit)
            if self.cycle_limit > 0 && *visit_count > self.cycle_limit {
                error!(
                    node_id = %current_node_id,
                    visit_count = %visit_count,
                    limit = %self.cycle_limit,
                    "Cycle limit exceeded in workflow execution"
                );
                return Err(WorkflowError::NodeExecution(
                    FloxideError::WorkflowCycleDetected,
                ));
            }

            let node = self.nodes.get(&current_node_id).ok_or_else(|| {
                error!(node_id = %current_node_id, "Node not found in workflow");
                WorkflowError::NodeNotFound(current_node_id.clone())
            })?;

            debug!(node_id = %current_node_id, visit_count = %visit_count, "Executing node");

            let outcome = node
                .process(ctx)
                .await
                .map_err(WorkflowError::NodeExecution)?;

            match &outcome {
                NodeOutcome::Success(_) => {
                    debug!(node_id = %current_node_id, "Node completed successfully with Success outcome");
                }
                NodeOutcome::Skipped => {
                    debug!(node_id = %current_node_id, "Node completed with Skipped outcome");
                }
                NodeOutcome::RouteToAction(action) => {
                    debug!(node_id = %current_node_id, action = %action.name(), action_debug = ?action, "Node completed with RouteToAction outcome");
                }
            }

            match outcome {
                NodeOutcome::Success(output) => {
                    debug!(node_id = %current_node_id, "Node completed successfully");
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
            allow_cycles: self.allow_cycles,
            cycle_limit: self.cycle_limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::closure;

    #[derive(Debug, Clone)]
    struct TestContext {
        value: i32,
        visited: Vec<String>,
    }

    #[tokio::test]
    async fn test_simple_linear_workflow() {
        // Create nodes
        let start_node = closure::node(|mut ctx: TestContext| async move {
            ctx.value += 1;
            ctx.visited.push("start".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let middle_node = closure::node(|mut ctx: TestContext| async move {
            ctx.value *= 2;
            ctx.visited.push("middle".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let end_node = closure::node(|mut ctx: TestContext| async move {
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
        let start_node = closure::node(|mut ctx: TestContext| async move {
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

        let path1_node = closure::node(|mut ctx: TestContext| async move {
            ctx.value += 100;
            ctx.visited.push("path1".to_string());
            Ok((ctx, NodeOutcome::<(), TestAction>::Success(())))
        });

        let path2_node = closure::node(|mut ctx: TestContext| async move {
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
        let start_node = closure::node(|mut ctx: TestContext| async move {
            ctx.visited.push("start".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let skip_node = closure::node(|mut ctx: TestContext| async move {
            ctx.visited.push("skip_check".to_string());
            if ctx.value > 5 {
                // Skip this node
                Ok((ctx, NodeOutcome::<(), DefaultAction>::Skipped))
            } else {
                ctx.value *= 2;
                Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
            }
        });

        let end_node = closure::node(|mut ctx: TestContext| async move {
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

    #[tokio::test]
    async fn test_cyclic_workflow() {
        // Create nodes
        let start_node = closure::node(|mut ctx: TestContext| async move {
            ctx.value += 1;
            ctx.visited.push("start".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let loop_node = closure::node(|mut ctx: TestContext| async move {
            ctx.value *= 2;
            ctx.visited.push("loop".to_string());

            // Continue looping until value > 100
            if ctx.value <= 100 {
                Ok((
                    ctx,
                    NodeOutcome::<(), DefaultAction>::RouteToAction(DefaultAction::Next),
                ))
            } else {
                Ok((
                    ctx,
                    NodeOutcome::<(), DefaultAction>::RouteToAction(DefaultAction::Error),
                ))
            }
        });

        let end_node = closure::node(|mut ctx: TestContext| async move {
            ctx.value -= 10;
            ctx.visited.push("end".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        // Build workflow with cycles allowed
        let mut workflow = Workflow::new(start_node);
        let start_id = workflow.start_node.clone();
        let loop_id = loop_node.id();
        let end_id = end_node.id();

        workflow
            .add_node(loop_node)
            .add_node(end_node)
            .set_default_route(&start_id, &loop_id)
            .connect(&loop_id, DefaultAction::Next, &loop_id) // Cycle back to loop_node
            .connect(&loop_id, DefaultAction::Error, &end_id)
            .allow_cycles(true) // Enable cycles
            .set_cycle_limit(10); // Set a reasonable limit

        // Execute workflow
        let mut ctx = TestContext {
            value: 3,
            visited: vec![],
        };

        let result = workflow.execute(&mut ctx).await;
        assert!(result.is_ok());

        // Check final state
        // Initial value: 3
        // After start: 3 + 1 = 4
        // Loop iterations:
        // 1: 4 * 2 = 8
        // 2: 8 * 2 = 16
        // 3: 16 * 2 = 32
        // 4: 32 * 2 = 64
        // 5: 64 * 2 = 128 (> 100, so route to end)
        // After end: 128 - 10 = 118
        assert_eq!(ctx.value, 118);

        // Should have visited start once, loop 5 times, and end once
        assert_eq!(ctx.visited.len(), 7);
        assert_eq!(ctx.visited[0], "start");
        assert_eq!(ctx.visited[1], "loop");
        assert_eq!(ctx.visited[2], "loop");
        assert_eq!(ctx.visited[3], "loop");
        assert_eq!(ctx.visited[4], "loop");
        assert_eq!(ctx.visited[5], "loop");
        assert_eq!(ctx.visited[6], "end");

        // Test with cycles disabled - create new nodes
        let start_node2 = closure::node(|mut ctx: TestContext| async move {
            ctx.value += 1;
            ctx.visited.push("start".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        let loop_node2 = closure::node(|mut ctx: TestContext| async move {
            ctx.value *= 2;
            ctx.visited.push("loop".to_string());

            // Continue looping until value > 100
            if ctx.value <= 100 {
                Ok((
                    ctx,
                    NodeOutcome::<(), DefaultAction>::RouteToAction(DefaultAction::Next),
                ))
            } else {
                Ok((
                    ctx,
                    NodeOutcome::<(), DefaultAction>::RouteToAction(DefaultAction::Error),
                ))
            }
        });

        let end_node2 = closure::node(|mut ctx: TestContext| async move {
            ctx.value -= 10;
            ctx.visited.push("end".to_string());
            Ok((ctx, NodeOutcome::<(), DefaultAction>::Success(())))
        });

        // Build workflow with cycles disabled
        let mut workflow2 = Workflow::new(start_node2);
        let start_id2 = workflow2.start_node.clone();
        let loop_id2 = loop_node2.id();
        let end_id2 = end_node2.id();

        workflow2
            .add_node(loop_node2)
            .add_node(end_node2)
            .set_default_route(&start_id2, &loop_id2)
            .connect(&loop_id2, DefaultAction::Next, &loop_id2) // Cycle back to loop_node
            .connect(&loop_id2, DefaultAction::Error, &end_id2)
            .allow_cycles(false); // Disable cycles

        let mut ctx2 = TestContext {
            value: 3,
            visited: vec![],
        };

        let result2 = workflow2.execute(&mut ctx2).await;
        assert!(result2.is_err());

        // Should have detected a cycle after the first loop iteration
        match result2 {
            Err(WorkflowError::NodeExecution(FloxideError::WorkflowCycleDetected)) => {
                // This is the expected error
            }
            _ => panic!("Expected WorkflowCycleDetected error, got {:?}", result2),
        }

        // Should have visited start once and loop once
        assert_eq!(ctx2.visited.len(), 2);
        assert_eq!(ctx2.visited[0], "start");
        assert_eq!(ctx2.visited[1], "loop");
    }
}
