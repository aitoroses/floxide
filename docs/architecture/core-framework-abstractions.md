# Core Framework Abstractions

This document describes the core abstractions that form the foundation of the Flowrs framework.

## Overview

The Flowrs framework is designed as a directed graph workflow system built on several key abstractions:

1. **Node Interface**: The core building block for workflow steps
2. **Action Types**: Type-safe transitions between nodes
3. **Context**: State container passed between nodes
4. **Workflow**: The orchestrator that manages node execution
5. **Batch Processing**: Capability for parallel execution

These abstractions work together to create a flexible, type-safe, and composable workflow system.

## Node Interface

The Node trait is the fundamental building block of the Flowrs framework. It defines the lifecycle methods that all nodes must implement:

```rust
pub trait Node: Send + Sync + 'static {
    /// The context type this node operates on
    type Context: Send + 'static;

    /// Prepare the node for execution (optional)
    fn prep(&self, _ctx: &mut Self::Context) -> NodeResult {
        Ok(())
    }

    /// Execute the node's main logic
    fn exec(&self, ctx: &mut Self::Context) -> NodeResult;

    /// Perform post-execution cleanup (optional)
    fn post(&self, _ctx: &mut Self::Context) -> NodeResult {
        Ok(())
    }
}
```

The Node trait follows a three-phase lifecycle:

1. **Preparation (`prep`)**: Optional setup before execution
2. **Execution (`exec`)**: The main operation of the node
3. **Post-processing (`post`)**: Optional cleanup after execution

## Action Types

Actions define the transitions between nodes in a workflow. The Flowrs framework uses a trait-based approach for actions that allows for type-safe, domain-specific action types:

```rust
/// Trait for types that can be used as actions in workflow transitions
pub trait ActionType: Debug + Clone + PartialEq + Eq + Hash + Send + Sync + 'static {}

/// Standard action types provided by the framework
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DefaultAction {
    /// Default transition to the next node
    Next,
    /// Successfully complete the workflow
    Complete,
    /// Signal an error condition
    Error,
}

impl ActionType for DefaultAction {}
```

This approach allows users to define their own custom action types that are fully type-safe at compile time.

## Context

Contexts are containers for state that is passed between nodes during workflow execution. The basic Context type is generic over the state it contains:

```rust
pub struct Context<S> {
    state: S,
}

impl<S> Context<S> {
    /// Create a new context with the given state
    pub fn new(state: S) -> Self {
        Self { state }
    }

    /// Get a reference to the state
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Get a mutable reference to the state
    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }
}
```

Specialized context types like `BatchContext` and `EventContext` extend this basic context with additional functionality.

## Workflow

The Workflow struct is the orchestrator that manages the execution of nodes. It maintains a directed graph of nodes and handles the flow of execution based on the actions returned by nodes:

```rust
pub struct Workflow<A: ActionType, C> {
    nodes: HashMap<NodeId, Box<dyn Node<Context = C>>>,
    transitions: HashMap<(NodeId, A), NodeId>,
    start_node: Option<NodeId>,
}
```

The Workflow provides methods for:

- Adding nodes to the workflow
- Defining transitions between nodes
- Executing the workflow from start to finish
- Handling errors and retries

## Batch Processing

The batch processing capability allows for parallel execution of workflow steps on multiple items. It extends the core abstractions with:

- `BatchContext`: A specialized context that manages a collection of items
- `BatchNode`: A trait for nodes that can process items in parallel
- `BatchWorkflow`: An orchestrator for batch processing workflows

## Conclusion

These core abstractions provide the foundation for the Flowrs framework. By leveraging Rust's type system and ownership model, they enable the creation of robust, type-safe, and composable workflow applications.

For more detailed information on these abstractions, refer to the [Core Framework Abstractions ADR](../adrs/0003-core-framework-abstractions.md).
