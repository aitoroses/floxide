# ADR-0004: Async Runtime Selection

## Status

Accepted

## Date

2024-02-25

## Context

The flowrs framework is designed around asynchronous operations to efficiently handle workflow execution. In Rust, async operations require an explicit runtime to execute futures.

There are several viable async runtimes available in the Rust ecosystem, each with different trade-offs:

1. [Tokio](https://tokio.rs/): Full-featured, production-ready, widely adopted
2. [async-std](https://async.rs/): Similar to the standard library, focused on ergonomics
3. [smol](https://github.com/smol-rs/smol): Small and simple runtime
4. Custom runtimes: Roll-our-own or specialized solutions

We need to select an appropriate async runtime that will support the framework's execution model, particularly for parallel node execution and orchestration.

## Decision

We will use **Tokio** as the primary async runtime for the flowrs framework implementation, with full feature set enabled. Additionally, we will use the `async_trait` crate for trait methods that return futures.

### Implementation Details

1. **Core Runtime Usage**:

   ```rust
   // In flowrs-transform crate
   pub fn run_flow<S, F>(flow: F, shared_state: &mut S) -> Result<(), FlowrsError>
   where
       F: BaseNode<S>,
   {
       let rt = tokio::runtime::Runtime::new()?;
       rt.block_on(async {
           flow.run(shared_state).await
       })
   }
   ```

2. **Async Trait Implementation**:

   ```rust
   use async_trait::async_trait;

   #[async_trait]
   pub trait Node<Context, A = DefaultAction>
   where
       A: ActionType,
   {
       type Output;

       async fn process(&self, ctx: &mut Context) -> Result<NodeOutcome<Self::Output, A>, FlowrsError>;
   }
   ```

3. **Parallelism for BatchFlow**:

   ```rust
   // In BatchFlow implementation
   async fn exec_core(&self, prep_results: Vec<I>) -> Result<Vec<Result<(), FlowrsError>>, FlowrsError> {
       let mut handles = Vec::with_capacity(prep_results.len());

       for item in prep_results {
           // Create a cloned shared state for each parallel execution
           let state_clone = /* clone shared state */;
           let start_node = self.flow.get_start_node()?;

           let handle = tokio::spawn(async move {
               start_node.run(&mut state_clone).await
           });

           handles.push(handle);
       }

       let mut results = Vec::with_capacity(handles.len());
       for handle in handles {
           results.push(handle.await.unwrap_or_else(|e| Err(FlowrsError::JoinError(e.to_string()))));
       }

       Ok(results)
   }
   ```

4. **Configurable Runtime Options**:

   ```rust
   pub struct FlowrsRuntimeConfig {
       worker_threads: Option<usize>,
       thread_name_prefix: String,
       thread_stack_size: Option<usize>,
   }

   impl Default for FlowrsRuntimeConfig {
       fn default() -> Self {
           Self {
               worker_threads: None, // Use Tokio default
               thread_name_prefix: "flowrs-worker-".to_string(),
               thread_stack_size: None, // Use Tokio default
           }
       }
   }

   pub fn create_runtime(config: FlowrsRuntimeConfig) -> Result<tokio::runtime::Runtime, FlowrsError> {
       let mut rt_builder = tokio::runtime::Builder::new_multi_thread();

       if let Some(threads) = config.worker_threads {
           rt_builder.worker_threads(threads);
       }

       rt_builder.thread_name(config.thread_name_prefix);

       if let Some(stack_size) = config.thread_stack_size {
           rt_builder.thread_stack_size(stack_size);
       }

       rt_builder.enable_all()
           .build()
           .map_err(|e| FlowrsError::RuntimeCreationError(e.to_string()))
   }
   ```

5. **Abstraction Layer**:
   - We will create a runtime abstraction layer in the `flowrs-transform` crate
   - This will allow for potential future runtime switching
   - The public API will remain stable even if the underlying runtime changes

### Feature Flags

We will use feature flags to allow custom runtime configuration:

```toml
# In flowrs-transform/Cargo.toml
[features]
default = ["tokio-full"]
tokio-full = ["tokio/full"]
tokio-minimal = ["tokio/rt", "tokio/sync"]
custom-runtime = []

[dependencies]
tokio = { version = "1.36", features = ["full"] }
async-trait = "0.1.77"
```

## Consequences

### Positive

1. **Ecosystem Compatibility**: Tokio is the most widely used async runtime in Rust, providing compatibility with a large ecosystem of libraries
2. **Production-Ready**: Tokio is battle-tested and used in many production environments
3. **Feature-Rich**: Includes timers, I/O utilities, and synchronization primitives that will be useful for the framework
4. **Active Maintenance**: Actively developed and maintained
5. **Scalability**: Well-suited for high-performance, concurrent workloads
6. **Async Trait Support**: Using `async_trait` simplifies writing async methods in traits

### Negative

1. **Runtime Dependency**: Creates a dependency on a specific runtime
2. **Binary Size**: Tokio with full features adds to the binary size of applications using the framework
3. **Learning Curve**: Tokio has its own patterns and concepts to learn
4. **Opinionated**: Some design decisions in Tokio may not align perfectly with all use cases
5. **Macro Overhead**: `async_trait` adds some runtime overhead compared to native async traits (which aren't stable yet)

## Alternatives Considered

### 1. async-std

- **Pros**:
  - Familiar API that mirrors the standard library
  - Good documentation
  - Focuses on ergonomics
- **Cons**:
  - Less widely adopted than Tokio
  - Smaller ecosystem of compatible libraries
  - Some performance differences compared to Tokio

### 2. smol

- **Pros**:
  - Minimal footprint
  - Simple API
  - Lightweight
- **Cons**:
  - Less feature-rich
  - Smaller ecosystem
  - Less battle-tested in large-scale production environments

### 3. Runtime Agnostic Design

- **Pros**:
  - Maximum flexibility for consumers
  - No runtime dependency
- **Cons**:
  - Significantly more complex implementation
  - Would require extensive abstraction layers
  - Would limit the use of runtime-specific features

### 4. Allow Pluggable Runtimes

- **Pros**:
  - Flexibility for different environments
  - Could adapt to special requirements
- **Cons**:
  - Increased maintenance burden
  - More complex API
  - Testing complexity increases exponentially

### 5. Wait for native async traits

- **Pros**:
  - No need for `async_trait` macro
  - Better performance
  - More idiomatic Rust
- **Cons**:
  - Feature is not stable yet
  - Would delay development
  - Migration cost when the feature stabilizes

We chose Tokio with full features and `async_trait` because it provides the best balance of features, ecosystem compatibility, and production readiness. The abstraction layer will help mitigate some of the downsides by allowing for potential future changes without disrupting the API.
