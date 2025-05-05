//! # Floxide: Distributed Workflow Framework for Rust
//!
//! **Floxide** is a professional, extensible framework for building distributed, parallel, and event-driven workflows in Rust. It enables you to model complex business processes, data pipelines, and automation tasks as directed graphs, with robust support for distributed execution, checkpointing, and custom node logic.
//!
//! ## Key Features
//!
//! - **Distributed Execution**: Run workflows across multiple workers or nodes, with in-memory or pluggable backends for queues and checkpointing.
//! - **Parallelism**: Express parallel branches and concurrent processing natively in your workflow graphs.
//! - **Type-Safe Nodes**: Each node defines its own input/output types, ensuring correctness at compile time.
//! - **Checkpointing & Recovery**: Built-in support for checkpointing workflow state, enabling fault tolerance and resumability.
//! - **Declarative Workflow Definition**: Use the `workflow!` and `node!` macros to define nodes, edges, and transitions in a clear, maintainable way.
//! - **Production-Ready**: Designed for reliability, observability, and integration with async runtimes and tracing.
//!
//! ## Example: Distributed Parallel Workflow
//!
//! ```rust
//! use floxide::{workflow, node, Transition, WorkflowCtx, FloxideError};
//! use async_trait::async_trait;
//! use std::sync::Arc;
//! use tokio::sync::Mutex;
//!
//! #[derive(Clone, Debug)]
//! struct Ctx { counter: Arc<Mutex<i32>> }
//!
//! // Define a node that increments the counter
//! node! {
//!     pub struct StartNode {};
//!     context = Ctx;
//!     input = ();
//!     output = ();
//!     |ctx, _input| {
//!         let mut c = ctx.counter.lock().await;
//!         *c += 1;
//!         Ok(Transition::Next(()))
//!     }
//! }
//!
//! // Define a branch node that increments the counter by 10
//! node! {
//!     pub struct BranchNode {};
//!     context = Ctx;
//!     input = ();
//!     output = &'static str;
//!     |ctx, _input| {
//!         let mut c = ctx.counter.lock().await;
//!         *c += 10;
//!         Ok(Transition::Next("done"))
//!     }
//! }
//!
//! workflow! {
//!     pub struct ExampleWorkflow {
//!         start: StartNode,
//!         a: BranchNode,
//!         b: BranchNode,
//!     }
//!     start = start;
//!     context = Ctx;
//!     edges {
//!         start => { [ a, b ] };
//!         a => {};
//!         b => {};
//!     }
//! }
//! ```
//!
//! ## Getting Started
//!
//! Add Floxide to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! floxide = "1.0.0"
//! ```
//!
//! For full examples and advanced usage, see the `examples/` directory and the project documentation.
//!

// Re-export the core module (always included)
pub use ::floxide_macros::*;
pub use floxide_core::*;
