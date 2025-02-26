# ADR-0012: Testing Patterns for Async Node Implementations

## Status

Accepted

## Date

2024-02-26

## Context

Testing async code in Rust presents several challenges, particularly when it comes to:

1. Lifetime issues with closures that capture variables
2. Type inference complexities with async closures
3. Implementation constraints when using trait objects
4. Testing behavior that relies on future resolution

We encountered specific issues when testing our lifecycle node implementation:

- Lifetime errors when using closures directly in tests
- Difficulty creating reusable test patterns
- Readability and maintainability of test code

## Decision

We will adopt a set of testing patterns specifically for our async node implementations:

### 1. Use Concrete Implementations for Testing

Instead of using closures and the helper functions like `lifecycle_node()`, we'll create concrete test implementations of the traits:

```rust
// For testing LifecycleNode
struct TestLifecycleNode {
    id: NodeId,
}

#[async_trait]
impl LifecycleNode<TestContext, DefaultAction> for TestLifecycleNode {
    type PrepOutput = i32;
    type ExecOutput = i32;

    fn id(&self) -> NodeId { self.id.clone() }

    async fn prep(&self, ctx: &mut TestContext) -> Result<i32, FloxideError> {
        // Test-specific implementation...
        Ok(42)
    }

    // ... other method implementations
}
```

### 2. Test Adapters Directly

Test adapter implementations directly rather than through helper functions:

```rust
#[tokio::test]
async fn test_lifecycle_node() {
    let lifecycle_node = TestLifecycleNode { id: "test-node".to_string() };
    let node = LifecycleNodeAdapter::new(lifecycle_node);

    // Test node behavior...
}
```

### 3. Create Factory Functions for Complex Setup

For tests requiring complex setup, use factory functions that return fully initialized test objects:

```rust
fn create_test_workflow() -> Workflow<TestContext, DefaultAction> {
    let start_node = TestNode { id: "start".to_string() };
    let mut workflow = Workflow::new(start_node);
    // Add more nodes, configure workflow
    workflow
}
```

### 4. Use Type Aliases for Complex Types

When working with complex generic types, define type aliases to improve readability:

```rust
type TestWorkflow = Workflow<TestContext, DefaultAction, i32>;
type TestBatchNode = BatchNode<TestBatchContext, Item, DefaultAction>;
```

## Consequences

### Advantages

1. **No Lifetime Issues**: By using concrete implementations, we avoid the lifetime issues common with closures
2. **Clear Test Intent**: Tests are more explicit about what they're testing
3. **Better Test Organization**: Test objects can be reused across multiple tests
4. **Easier Debugging**: When tests fail, it's clearer where the failure occurs
5. **Isolated Test Logic**: Each test component has a clear responsibility

### Disadvantages

1. **More Boilerplate**: Requires more code to set up tests
2. **Lower Test-to-Code Ratio**: Tests may be significantly longer than the code they test
3. **Learning Curve**: New team members need to understand the testing patterns

## Alternatives Considered

### Using `Box<dyn Fn...>` for Lifecycle Closures

We considered changing the `lifecycle_node` function to accept boxed closures:

```rust
pub fn lifecycle_node<Context, Action, PrepOut, ExecOut>(
    id: Option<String>,
    prep_fn: Box<dyn Fn(&mut Context) -> BoxFuture<'_, Result<PrepOut, FloxideError>> + Send + Sync>,
    // ...
)
```

This would allow for easier use in tests, but would make the API more cumbersome for normal use.

### Testing Helper Functions Directly

We considered writing tests specifically for helper functions like `lifecycle_node`, which would allow simpler test cases. However, this wouldn't test the integration with the Node trait.

### Using Helper Macros for Tests

We explored creating test macros that would handle the boilerplate, but this would hide important details and make debugging more difficult.

## Implementation Notes

- We will update existing tests to follow these patterns
- We will document these patterns in the project's testing guidelines
- Test modules will be organized to match the structure of the code they test
- Helper modules may be created for shared test components
- Test coverage should focus on behavior, not implementation details
