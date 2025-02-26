# Async Runtime Selection

This document describes the async runtime selection for the Floxide framework.

## Overview

The Floxide framework is designed around asynchronous operations to efficiently handle workflow execution. In Rust, async operations require an explicit runtime to execute futures. This document explains the choice of async runtime and how it's used in the framework.

## Async Runtime Options

There are several viable async runtimes available in the Rust ecosystem, each with different trade-offs:

1. **Tokio**: Full-featured, production-ready, widely adopted
2. **async-std**: Similar to the standard library, focused on ergonomics
3. **smol**: Small and simple runtime
4. **Custom runtimes**: Roll-our-own or specialized solutions

## Selected Runtime: Tokio

The Floxide framework uses **Tokio** as its primary async runtime, with the full feature set enabled. Tokio was selected for several reasons:

1. **Maturity and Stability**: Tokio is a mature, production-ready runtime with a stable API.
2. **Performance**: Tokio offers excellent performance characteristics for the types of workloads the framework handles.
3. **Ecosystem Integration**: Tokio has broad adoption and integrates well with many other Rust crates.
4. **Feature Set**: Tokio provides a comprehensive set of features including timers, task scheduling, and I/O operations.
5. **Community Support**: Tokio has a large community and extensive documentation.

## Implementation Details

### Core Runtime Usage

The framework uses Tokio's runtime to execute async workflows:

```rust
// In floxide-transform crate
pub fn run_flow<S, F>(flow: F, shared_state: &mut S) -> Result<(), FloxideError>
where
    F: BaseNode<S>,
{
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        flow.run(shared_state).await
    })
}
```

### Async Trait Implementation

The framework uses the `async_trait` crate to support async methods in traits:

```rust
use async_trait::async_trait;

#[async_trait]
pub trait AsyncNode: Send + Sync + 'static {
    type Context: Send + 'static;

    async fn exec_async(&self, ctx: &mut Self::Context) -> NodeResult;
}
```

### Concurrency Control

Tokio's features are used for concurrency control in batch processing:

```rust
use tokio::sync::Semaphore;

pub struct BatchProcessor {
    concurrency_limit: usize,
    semaphore: Semaphore,
}

impl BatchProcessor {
    pub async fn process_batch<T, F, Fut>(&self, items: Vec<T>, processor: F) -> Vec<Result<T, Error>>
    where
        F: Fn(T) -> Fut + Send + Sync,
        Fut: Future<Output = Result<T, Error>> + Send,
        T: Send + 'static,
    {
        let mut handles = Vec::with_capacity(items.len());

        for item in items {
            let permit = self.semaphore.acquire().await.unwrap();
            let processor = &processor;

            let handle = tokio::spawn(async move {
                let result = processor(item).await;
                drop(permit);
                result
            });

            handles.push(handle);
        }

        // Collect results...
    }
}
```

## Runtime Configuration

The framework allows for configuration of the Tokio runtime:

```rust
pub struct RuntimeConfig {
    worker_threads: Option<usize>,
    thread_name: Option<String>,
    thread_stack_size: Option<usize>,
}

impl RuntimeConfig {
    pub fn build_runtime(&self) -> Result<Runtime, FloxideError> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();

        if let Some(threads) = self.worker_threads {
            builder.worker_threads(threads);
        }

        if let Some(name) = &self.thread_name {
            builder.thread_name(name);
        }

        if let Some(stack_size) = self.thread_stack_size {
            builder.thread_stack_size(stack_size);
        }

        builder.enable_all()
            .build()
            .map_err(|e| FloxideError::RuntimeCreationFailed(e.to_string()))
    }
}
```

## Best Practices

When working with async code in the Floxide framework, consider these best practices:

1. **Avoid Blocking Operations**: Don't perform blocking operations directly in async functions. Use `tokio::task::spawn_blocking` for CPU-intensive or blocking operations.
2. **Proper Error Handling**: Use the `?` operator or explicit error handling in async functions.
3. **Cancellation Safety**: Ensure that your async code handles cancellation gracefully.
4. **Resource Management**: Use RAII patterns to ensure resources are properly cleaned up, even if tasks are cancelled.
5. **Concurrency Limits**: Use semaphores or other mechanisms to limit concurrency when appropriate.

## Conclusion

The selection of Tokio as the async runtime for the Floxide framework provides a solid foundation for building efficient, concurrent workflow applications. By leveraging Tokio's features and the `async_trait` crate, the framework offers a powerful and flexible approach to asynchronous workflow execution.

For more detailed information on the async runtime selection, refer to the [Async Runtime Selection ADR](../adrs/0004-async-runtime-selection.md).
