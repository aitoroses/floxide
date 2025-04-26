# Floxide 🦀: The Power of Workflows in Rust

[![Crates.io](https://img.shields.io/crates/v/floxide-core.svg)](https://crates.io/crates/floxide-core)
[![Documentation](https://docs.rs/floxide-core/badge.svg)](https://docs.rs/floxide-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Floxide** is an extensible framework for building distributed, parallel, and event-driven workflows in Rust. Model complex processes as type-safe, directed graphs with robust support for:

*   **Distributed Execution:** Run workflows across multiple workers.
*   **Checkpointing & Recovery:** Fault tolerance and resumability.
*   **Declarative Definition:** Use `workflow!` and `node!` macros for clear definitions.
*   **Async Native:** Built for modern async Rust.

## Core Concepts

Floxide models workflows using these key components:

*   **`Node`**: A single, reusable step in a workflow. Defined using the `node!` macro.
*   **`Workflow`**: The overall directed graph structure, connecting Nodes. Defined using the `workflow!` macro.
*   **`Transition`**: The result of a Node's execution, indicating what happens next (`Next`, `NextAll`, `Hold`, `Abort`).
*   **`WorkflowCtx` / `Context`**: Shared data or state accessible by all Nodes within a specific workflow run.
*   **`WorkQueue`**: (For distributed workflows) A queue holding tasks (`WorkItem`s) ready to be processed by Workers.
*   **`Checkpoint`**: (For distributed workflows) Saved state of a workflow run, enabling recovery.
*   **`DistributedWorker`**: A process that dequeues tasks from the `WorkQueue` and executes Nodes.
*   **`DistributedOrchestrator`**: Manages the lifecycle of distributed workflow runs.

## Quick Example

Here's a conceptual workflow simulating article generation using LLM-like steps:

```rust
use floxide::{workflow, node, Transition, WorkflowCtx, FloxideError};
use async_trait::async_trait;

// Context: Shared data for the workflow (none needed here)
type Ctx = ();

// --- Node 1: Generate Outline --- 
node! {
    pub struct OutlineNode {};
    context = Ctx;
    input = String; // Input: Article Topic
    output = String; // Output: Outline Text
    | _ctx: &Ctx, topic: String | {
        println!("OutlineNode: Generating outline for topic: '{}'", topic);
        // Simulate calling an LLM to generate an outline
        let outline = format!("Outline for {}:\n- Introduction\n- Section 1\n- Section 2\n- Conclusion", topic);
        // Pass the outline to the next step
        Ok(Transition::Next(outline))
    }
}

// --- Node 2: Draft Article --- 
node! {
    pub struct DraftNode {};
    context = Ctx;
    input = String; // Input: Outline Text
    output = String; // Output: Draft Article Text
    | _ctx: &Ctx, outline: String | {
        println!("DraftNode: Drafting article based on outline...");
        // Simulate calling an LLM to draft based on the outline
        let draft = format!("Draft based on {}\n\n[... Full draft content based on outline sections ...]", outline);
        // Pass the draft to the next step
        Ok(Transition::Next(draft))
    }
}

// --- Node 3: Review Article --- 
node! {
    pub struct ReviewNode {};
    context = Ctx;
    input = String; // Input: Draft Article Text
    output = String; // Output: Final Article Text
    | _ctx: &Ctx, draft: String | {
        println!("ReviewNode: Reviewing and finalizing draft...");
        // Simulate a review step (e.g., adding a title, minor edits)
        let final_article = format!("**Final Article**\n\n{}", draft.replace("Draft based on", "Article based on"));
        // Pass the final article as the workflow result
        Ok(Transition::Next(final_article))
    }
}

// --- Workflow Definition: Connecting the nodes ---
workflow! {
    pub st`uct ArticleWriterWorkflow {
        outline: OutlineNode,
        draft: DraftNode,
        review: ReviewNode,
    }
    start = outline; // Start with the outline node
    context = Ctx;
    edges {
        // Define the sequence: outline -> draft -> review
        outline => draft;
        draft => review;
        review => {}; // review is the final node
    }
}

// Note: Running this workflow requires setting up an executor and providing 
// the initial topic input. See the tutorials for complete examples.
```
## Examples

The `examples/` directory contains various demonstrations of Floxide's features:

*   **`node_macro_example.rs`**: Basic usage of the `node!` macro to define a node with internal state and custom context.
*   **`simple_context_example.rs`**: Demonstrates a workflow with a shared context (`MyCtx`) and composite nodes that branch based on an enum output (`FooAction`).
*   **`split_example.rs`**: Shows how to use `SplitNode` to fan-out a single input into multiple items for parallel processing.
*   **`merge_example.rs`**: Complements `split_example` by using a custom `MergeNode` to collect results from parallel branches, holding until all expected inputs arrive.
*   **`batch_example.rs`**: Demonstrates `BatchNode` for processing items in groups, followed by routing based on batch results.
*   **`retry_example.rs`**: Using `RetryNode` (via `with_retry`) to wrap a node that might fail transiently, applying a retry policy.
*   **`retry_macro_example.rs`**: Defining retry policies directly within the `workflow!` macro using the `#[retry = ...]` attribute.
*   **`error_fallback_macro.rs`**: Handling node failures at the workflow level using the `on_failure` clause in the `edges` block.
*   **`checkpoint_example.rs`**: Shows how to use `run_with_checkpoint` and `resume` with an `InMemoryCheckpointStore` for fault tolerance.
*   **`cancellation_example.rs`**: Demonstrates graceful workflow cancellation using the `cancel_token` from `WorkflowCtx`.
*   **`timeout_example.rs`**: Setting a timeout on the `WorkflowCtx` to automatically abort long-running workflows.
*   **`nested_workflow_example.rs`**: Embedding one workflow within another using `CompositeNode`.
*   **`generics_example.rs`**: Defining workflows with nodes that have generic type parameters.
*   **`timer_example.rs`**: Using a `source` node (backed by a channel) to drive a workflow with external events (like timer ticks).
*   **`distributed_example.rs`**: Simulates a distributed workflow run using in-memory components (`InMemoryWorkQueue`, `InMemoryCheckpointStore`) and multiple worker tasks.
*   **`distributed_orchestrated_merge_example.rs`**: A more complex distributed example showcasing `OrchestratorBuilder`, `WorkerBuilder`, `WorkerPool`, and various in-memory distributed stores (`RunInfoStore`, `MetricsStore`, etc.) for a split/merge workflow with potential failures.
*   **`workflow_dot.rs`**: Demonstrates generating a Graphviz DOT representation of a workflow's structure using the `to_dot()` method.
*   **`terminal_node_example.rs`**: A minimal workflow where the starting node is also the terminal node, directly returning the final result.

## Installation

Add Floxide to your `Cargo.toml` dependencies:

```toml
[dependencies]
floxide = "*" # Check crates.io for the latest version
```

## Getting Started

The best way to learn Floxide is through the documentation and tutorials:

*   **[Floxide Documentation & Tutorials](https://aitoroses.github.io/floxide/)**: Start here for comprehensive guides and examples.
    *   **[Core Concepts Tutorial](https://aitoroses.github.io/floxide/floxide/index/)**: Learn the fundamentals of Workflows, Nodes, Transitions, and Context.
    *   **[Distributed Tutorial](https://aitoroses.github.io/floxide/floxide-tutorial/index/)**: Dive into distributed execution with Workers, Orchestrators, Queues, and Checkpointing.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Floxide is licensed under the MIT License. See the [LICENSE](LICENSE) file for details. 