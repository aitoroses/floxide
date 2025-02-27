# ADR-0003: Core Framework Abstractions

## Status

Accepted

## Date

2025-02-27

## Context

The floxide framework is designed as a directed graph workflow system that needs several key abstractions:

1. A core node interface for workflow steps
2. A retry mechanism that handles failures
3. A directed graph structure for the workflow
4. A batch processing capability for parallel execution

To create a robust and flexible framework, we need to determine how to implement these abstractions in Rust, leveraging traits, enums, and Rust's ownership model.

## Decision

We will implement the core abstractions of the floxide framework using a more idiomatic Rust approach that emphasizes clear ownership, strong typing, and composition over inheritance.

### Core Abstractions

#### 1. Action Type

Instead of relying on string-based custom actions, we'll use a trait-based approach that allows users to define their own fully type-safe action types:

```rust
/// Trait for types that can be used as actions in workflow transitions
///
/// By implementing this trait for your own enums, you can define domain-specific
/// actions that are fully type-safe at compile time.
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

// Example of how users can define their own type-safe action types:
//
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum PaymentAction {
//     PaymentReceived,
//     PaymentDeclined,
//     RefundRequested,
//     RefundProcessed,
// }
//
// impl ActionType for PaymentAction {}
```

With this approach, users can define their own domain-specific action types that are fully checked at compile time, avoiding the runtime errors that could occur with string-based custom actions.

##### Example: Order Processing Workflow

Here's a practical example of how domain-specific action types can be used to model a real-world order processing workflow:

```rust
// Define domain-specific action types for order processing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OrderAction {
    Approved,
    Rejected,
    Shipped,
    Delivered,
    Returned,
}

impl ActionType for OrderAction {}

// Create a context type to hold order data
struct OrderContext {
    order_id: String,
    customer_id: String,
    items: Vec<OrderItem>,
    status: OrderStatus,
    // other order details...
}

// Create nodes for the workflow (implementation details omitted)
fn create_order_node() -> impl Node<OrderContext, OrderAction> {
    // Implementation...
    # node(|_ctx| async { Ok(NodeOutcome::Transition(OrderAction::Approved, ())) })
}

fn validate_order_node() -> impl Node<OrderContext, OrderAction> {
    // Implementation...
    # node(|ctx| async {
    #    // Validate order logic...
    #    if ctx.items.is_empty() {
    #        Ok(NodeOutcome::Transition(OrderAction::Rejected, ()))
    #    } else {
    #        Ok(NodeOutcome::Transition(OrderAction::Approved, ()))
    #    }
    # })
}

fn process_order_node() -> impl Node<OrderContext, OrderAction> {
    // Implementation...
    # node(|_ctx| async { Ok(NodeOutcome::Transition(OrderAction::Shipped, ())) })
}

fn ship_order_node() -> impl Node<OrderContext, OrderAction> {
    // Implementation...
    # node(|_ctx| async { Ok(NodeOutcome::Transition(OrderAction::Delivered, ())) })
}

fn deliver_order_node() -> impl Node<OrderContext, OrderAction> {
    // Implementation...
    # node(|_ctx| async { Ok(NodeOutcome::Complete(())) })
}

fn reject_order_node() -> impl Node<OrderContext, OrderAction> {
    // Implementation...
    # node(|_ctx| async { Ok(NodeOutcome::Complete(())) })
}

// Create a function that builds and returns the complete workflow
fn create_order_workflow() -> Workflow<OrderContext, OrderAction> {
    let mut workflow = Workflow::new(create_order_node());

    let validation_id = workflow.add_node(validate_order_node());
    let processing_id = workflow.add_node(process_order_node());
    let shipping_id = workflow.add_node(ship_order_node());
    let delivery_id = workflow.add_node(deliver_order_node());
    let rejection_id = workflow.add_node(reject_order_node());

    // Connect the nodes with type-safe transitions
    workflow.connect(workflow.entry_point, OrderAction::Approved, validation_id);
    workflow.connect(validation_id, OrderAction::Approved, processing_id);
    workflow.connect(validation_id, OrderAction::Rejected, rejection_id);
    workflow.connect(processing_id, OrderAction::Shipped, shipping_id);
    workflow.connect(shipping_id, OrderAction::Delivered, delivery_id);

    workflow
}

// Using the workflow
async fn process_new_order(order: Order) -> Result<(), FloxideError> {
    let mut context = OrderContext::from(order);
    let workflow = create_order_workflow();
    workflow.execute(&mut context).await
}
```

This example demonstrates:

1. Creating a domain-specific `OrderAction` enum with meaningful action names
2. Building a workflow that uses these type-safe actions for transitions
3. Clear self-documenting code where the action names express business logic
4. Compiler-enforced correctness (e.g., can't accidentally use `PaymentAction` in an order workflow)

#### 2. Node Outcome

Instead of multiple lifecycle methods (prepare, execute, finalize), we'll use a single method with an enum return type that represents the outcome:

```rust
/// The result of processing a node
pub enum NodeOutcome<T, A = DefaultAction>
where
    A: ActionType,
{
    /// Node has completed processing with an output value
    Complete(T),
    /// Node wants to transition to another node via the specified action
    Transition(A, T),
}
```

#### 3. Node Trait

The core node functionality is defined as a Rust trait with a single, clear processing method:

```rust
/// Core trait representing a node in the workflow graph
pub trait Node<Context, A = DefaultAction>
where
    A: ActionType,
{
    /// The output type produced by this node
    type Output;

    /// Process this node with the given context
    async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, A>, FloxideError>;
}
```

#### 4. Workflow Structure

Instead of nodes knowing their successors, we'll use a dedicated workflow structure to manage the graph:

```rust
/// A unique identifier for nodes within a workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(uuid::Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// A workflow graph that connects nodes together
pub struct Workflow<Context, A = DefaultAction>
where
    A: ActionType,
{
    nodes: HashMap<NodeId, Box<dyn Node<Context, A>>>,
    edges: HashMap<(NodeId, A), NodeId>,
    entry_point: NodeId,
}

impl<Context, A> Workflow<Context, A>
where
    A: ActionType,
{
    /// Create a new workflow with the specified entry point node
    pub fn new(entry_node: impl Node<Context, A> + 'static) -> Self {
        let entry_id = NodeId::new();
        let mut nodes = HashMap::new();
        nodes.insert(entry_id, Box::new(entry_node));

        Self {
            entry_point: entry_id,
            nodes,
            edges: HashMap::new(),
        }
    }

    /// Add a node to the workflow and return its ID
    pub fn add_node(&mut self, node: impl Node<Context, A> + 'static) -> NodeId {
        let id = NodeId::new();
        self.nodes.insert(id, Box::new(node));
        id
    }

    /// Connect two nodes with a directed edge and an action
    pub fn connect(&mut self, from: NodeId, action: A, to: NodeId) -> &mut Self {
        self.edges.insert((from, action), to);
        self
    }

    /// Execute the workflow with the provided context
    pub async fn execute(&self, ctx: &mut Context) -> Result<(), FloxideError> {
        let mut current = self.entry_point;

        loop {
            let node = self.nodes.get(&current)
                .ok_or_else(|| FloxideError::NodeNotFound(format!("{:?}", current)))?;

            match node.process(ctx).await? {
                NodeOutcome::Complete(_) => return Ok(()),
                NodeOutcome::Transition(action, _) => {
                    current = *self.edges.get(&(current, action.clone()))
                        .ok_or_else(|| FloxideError::EdgeNotFound(
                            format!("{:?}", current),
                            format!("{:?}", action)
                        ))?;
                }
            }
        }
    }
}
```

#### 5. Retry Mechanism

We'll implement retry as a wrapper node that adds retry capability to any other node:

```rust
/// Strategy for timing retries
pub enum BackoffStrategy {
    /// No delay between retries
    Immediate,
    /// Fixed delay between retries
    Fixed(Duration),
    /// Exponential backoff with optional jitter
    Exponential {
        base_delay: Duration,
        max_delay: Duration,
        factor: f64,
        jitter: bool,
    },
}

/// Node wrapper that adds retry capability
pub struct RetryNode<N> {
    inner: N,
    max_retries: usize,
    backoff_strategy: BackoffStrategy,
}

impl<Context, A, N> Node<Context, A> for RetryNode<N>
where
    N: Node<Context, A>,
    A: ActionType,
{
    type Output = N::Output;

    async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, A>, FloxideError> {
        let mut attempts = 0;

        loop {
            match self.inner.process(ctx).await {
                Ok(outcome) => return Ok(outcome),
                Err(err) => {
                    attempts += 1;
                    if attempts >= self.max_retries {
                        return Err(err);
                    }

                    match &self.backoff_strategy {
                        BackoffStrategy::Immediate => {},
                        BackoffStrategy::Fixed(duration) => {
                            tokio::time::sleep(*duration).await;
                        },
                        BackoffStrategy::Exponential { base_delay, max_delay, factor, jitter } => {
                            let backoff = base_delay.mul_f64(factor.powi(attempts as i32));
                            let capped_backoff = std::cmp::min(backoff, *max_delay);

                            let actual_delay = if *jitter {
                                let jitter_factor = rand::random::<f64>() * 0.5 + 0.5; // 0.5 to 1.0
                                capped_backoff.mul_f64(jitter_factor)
                            } else {
                                capped_backoff
                            };

                            tokio::time::sleep(actual_delay).await;
                        }
                    }
                }
            }
        }
    }
}

// Helper methods for creating retry nodes
impl<N> RetryNode<N> {
    pub fn new(inner: N, max_retries: usize) -> Self {
        Self {
            inner,
            max_retries,
            backoff_strategy: BackoffStrategy::Immediate,
        }
    }

    pub fn with_fixed_backoff(inner: N, max_retries: usize, delay: Duration) -> Self {
        Self {
            inner,
            max_retries,
            backoff_strategy: BackoffStrategy::Fixed(delay),
        }
    }

    pub fn with_exponential_backoff(
        inner: N,
        max_retries: usize,
        base_delay: Duration,
        max_delay: Duration,
        factor: f64,
        jitter: bool,
    ) -> Self {
        Self {
            inner,
            max_retries,
            backoff_strategy: BackoffStrategy::Exponential {
                base_delay,
                max_delay,
                factor,
                jitter,
            },
        }
    }
}
```

#### 6. Batch Processing

We'll implement batch processing as a specialized node that processes items in parallel:

```rust
/// A node that processes a collection of items in parallel
pub struct BatchNode<ItemNode, ItemType, Context, A = DefaultAction>
where
    ItemNode: Node<Context, A>,
    A: ActionType,
{
    item_node: ItemNode,
    parallelism: usize,
    _phantom: PhantomData<(ItemType, Context, A)>,
}

impl<ItemNode, ItemType, Context, A> BatchNode<ItemNode, ItemType, Context, A>
where
    ItemNode: Node<Context, A> + Clone,
    A: ActionType,
{
    pub fn new(item_node: ItemNode, parallelism: usize) -> Self {
        Self {
            item_node,
            parallelism,
            _phantom: PhantomData,
        }
    }
}

impl<ItemNode, ItemType, Context, A> Node<Context, A> for BatchNode<ItemNode, ItemType, Context, A>
where
    ItemNode: Node<Context, A> + Clone + Send + Sync + 'static,
    ItemType: Send + Sync + 'static,
    Context: BatchContext<ItemType> + Send,
    A: ActionType,
{
    type Output = Vec<Result<ItemNode::Output, FloxideError>>;

    async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, A>, FloxideError> {
        let items = ctx.get_batch_items()?;

        let results = process_batch(
            items,
            self.parallelism,
            |item| {
                let node = self.item_node.clone();
                let mut item_ctx = ctx.create_item_context(item)?;

                async move {
                    match node.process(&mut item_ctx).await {
                        Ok(NodeOutcome::Complete(output)) => Ok(output),
                        Ok(NodeOutcome::Transition(_, output)) => Ok(output),
                        Err(e) => Err(e),
                    }
                }
            }
        ).await;

        Ok(NodeOutcome::Complete(results))
    }
}

/// Helper trait for contexts that support batch processing
pub trait BatchContext<T> {
    fn get_batch_items(&self) -> Result<Vec<T>, FloxideError>;
    fn create_item_context(&self, item: T) -> Result<Self, FloxideError> where Self: Sized;
}

async fn process_batch<T, F, Fut, R>(
    items: Vec<T>,
    parallelism: usize,
    process_fn: F,
) -> Vec<Result<R, FloxideError>>
where
    T: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<R, FloxideError>> + Send,
    R: Send + 'static,
{
    use futures::stream::{self, StreamExt};

    stream::iter(items)
        .map(|item| {
            let process = &process_fn;
            async move { process(item).await }
        })
        .buffer_unordered(parallelism)
        .collect::<Vec<_>>()
        .await
}
```

#### 7. Convenience Node Builders

We'll provide helper functions to create nodes from closures:

```rust
/// Create a simple node from an async function
pub fn node<Context, A, T, F, Fut>(f: F) -> impl Node<Context, A, Output = T>
where
    F: Fn(&mut Context) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<NodeOutcome<T, A>, FloxideError>> + Send + 'static,
    A: ActionType,
    T: 'static,
{
    struct SimpleNode<F, T, Context, A> {
        func: F,
        _phantom: PhantomData<(T, Context, A)>,
    }

    impl<F, T, Context, A, Fut> Node<Context, A> for SimpleNode<F, T, Context, A>
    where
        F: Fn(&mut Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<NodeOutcome<T, A>, FloxideError>> + Send + 'static,
        A: ActionType,
    {
        type Output = T;

        async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<T, A>, FloxideError> {
            (self.func)(ctx).await
        }
    }

    SimpleNode {
        func: f,
        _phantom: PhantomData,
    }
}
```

### Type Safety and Composition

This design emphasizes:

1. Clear separation between the node behavior (the `Node` trait) and the graph structure (the `Workflow` struct)
2. A single processing method instead of three lifecycle methods
3. Strong typing with enums for outcomes and actions
4. Composition through node wrappers rather than inheritance
5. Powerful retry strategies with various backoff options
6. Explicit node creation and connection rather than implicit knowledge of successors
7. Type-safe custom action types through user-defined enums that implement the `ActionType` trait

## Consequences

### Positive

1. **Idiomatic Rust**: Uses Rust's strengths like enums, traits, and composition
2. **Clear Ownership Model**: Explicit about who owns what
3. **Simpler API**: One method to implement instead of three
4. **Strong Type Safety**: Makes invalid states unrepresentable
5. **Separation of Concerns**: Nodes focus on processing, workflow manages connections
6. **Composable**: Easy to wrap nodes with additional functionality
7. **Expressive**: Outcome enums clearly express intents
8. **Builder Pattern**: Fluent API for constructing workflows
9. **Type-Safe Actions**: No string-based actions that could cause runtime errors

### Negative

1. **Learning Curve**: Different conceptual model than some may be familiar with
2. **Serialization Complexity**: Graph structure might be harder to serialize/deserialize
3. **Verbose Generics**: Some implementations have complex type parameters
4. **Runtime Type Information**: Still uses dynamic dispatch for heterogeneous nodes
5. **Multiple Action Types**: Workflows can only use one action type, which might require conversion between different action types

## Alternatives Considered

### 1. Multi-method Lifecycle Model

- **Pros**:
  - Separate phases of execution are explicit
  - Familiar pattern for those coming from OOP
- **Cons**:
  - More complex to implement correctly
  - Forces a specific execution model
  - Less idiomatic in Rust

### 2. Self-referential Nodes

- **Pros**:
  - Nodes can directly reference their successors
  - No need for external graph structure
- **Cons**:
  - Creates complex ownership issues in Rust
  - Harder to serialize/deserialize
  - Poor separation of concerns

### 3. Static Dispatch Approach

- **Pros**:
  - Better performance
  - No runtime overhead
- **Cons**:
  - Much more complex type parameters
  - Harder to compose nodes dynamically
  - Limited heterogeneous collections

### 4. Channels-based Communication

- **Pros**:
  - More actor-like model
  - Better isolation between nodes
- **Cons**:
  - More complex to reason about
  - Harder to debug
  - More overhead

### 5. String-Based Custom Actions

- **Pros**:
  - More dynamic and flexible at runtime
  - Easy to serialize/deserialize
- **Cons**:
  - No compile-time type checking
  - Prone to runtime errors from typos
  - Less performant due to string comparison
  - Not idiomatic Rust

We chose the outcome-based node design with a separate workflow structure and user-defined action types because it provides a good balance of idiomatic Rust, type safety, and usability while maintaining the flexibility needed for a workflow system. The approach emphasizes composition over inheritance and makes excellent use of Rust's strengths in algebraic data types and ownership.
