# ADR-0011: Closure Lifetime Management in Async Contexts

## Status

Accepted

## Date

2025-02-27

## Context

We're encountering lifetime issues when using closures in async contexts, particularly in our `lifecycle_node` function. The problem occurs because:

1. The closures capture references to local variables
2. The async blocks created from these closures must satisfy lifetime bounds
3. The compiler can't guarantee that the references will live long enough when the Future is awaited

Specifically, we're seeing errors like:

```
lifetime may not live long enough
returning this value requires that `'1` must outlive `'2`
```

This is a common issue in Rust's async ecosystem when closures capture references and are then returned from functions.

## Decision

We will adopt a multi-faceted approach to handling closure lifetimes in async contexts:

### 1. Use 'static Types for Closure Inputs and Outputs

Any data passed into or out of closures used in async contexts will be required to satisfy the 'static lifetime bound. This ensures the data will live for the entire program duration:

```rust
where
    Context: Send + Sync + 'static,
    Action: ActionType + Send + Sync + 'static,
    PrepOut: Send + Sync + Clone + 'static,
    ExecOut: Send + Sync + Clone + 'static,
```

### 2. Explicitly Use `move` Closures

We'll always use the `move` keyword when defining closures that will be used in async contexts to ensure ownership is transferred into the closure:

```rust
move |ctx: &mut TestContext| async move {
    // Closure body
}
```

### 3. Document Function-specific Lifetime Requirements

For functions that take closures, we'll document that the closures:

1. Must be `move` closures
2. Any captured data must meet the 'static lifetime bound
3. References passed as parameters follow the normal borrowing rules

### 4. Create Helper Types for Complex Cases

In cases where we need to capture references with specific lifetimes, we'll create dedicated structs with explicit lifetime parameters:

```rust
struct ContextBorrower<'a, T> {
    context: &'a mut T,
}

impl<'a, T> ContextBorrower<'a, T> {
    async fn process_with_context<F, Fut>(&mut self, f: F) -> Fut::Output
    where
        F: FnOnce(&mut T) -> Fut,
        Fut: Future,
    {
        f(self.context).await
    }
}
```

## Consequences

### Advantages

1. **Type Safety**: The compiler enforces our lifetime constraints
2. **Clear Requirements**: Using `move` consistently makes ownership transfer explicit
3. **Reduced Bugs**: Avoids subtle lifetime bugs that could manifest at runtime
4. **Better Composability**: Working with 'static data makes composition easier

### Disadvantages

1. **More Constraints**: Requires data to be owned or 'static
2. **Additional Complexity**: May require additional cloning in some cases
3. **Learning Curve**: Developers need to understand the reasons for these patterns

## Alternatives Considered

### Static Function References Instead of Closures

We considered using static function references (`fn() -> ...`) instead of closures, but this would severely limit the expressiveness of our API.

### Allocating Contexts on the Heap

We explored allocating all context data on the heap with `Box` or `Arc`, but this complicates the API and introduces unnecessary allocation overhead.

### Returning Impl Future with Explicit Lifetimes

We investigated returning `impl Future + 'a` with explicit lifetimes, but this propagates lifetime complexity throughout the API and is difficult to implement correctly.

## Implementation Notes

- We'll update all tests to use `move` closures consistently
- Documentation will explicitly mention the need for `move` in async contexts
- Examples will demonstrate correct lifetime handling patterns
- We'll consider introducing a linter rule to enforce `move` for async closures
