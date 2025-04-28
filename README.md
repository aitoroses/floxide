# Floxide 🦀: The Power of Workflows in Rust

[![Crates.io](https://img.shields.io/crates/v/floxide.svg)](https://crates.io/crates/floxide)
[![Documentation](https://docs.rs/floxide/badge.svg)](https://docs.rs/floxide)
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
use floxide::{node, workflow, FloxideError, Node, Transition, Workflow, WorkflowCtx};
use rllm::{
    builder::{LLMBackend, LLMBuilder},
    chat::{ChatMessage, ChatRole, MessageType},
    LLMProvider,
};
use std::sync::Arc;
use std::{
    env,
    fmt::{self, Debug},
};
use tracing::Level;

#[derive(Clone)]
pub struct LLMProviderWrapper {
    inner: Arc<Box<dyn LLMProvider>>,
}

impl LLMProviderWrapper {
    pub fn new(inner: Arc<Box<dyn LLMProvider>>) -> Self {
        Self { inner }
    }
}

impl Debug for LLMProviderWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LLMProviderWrapper")
    }
}

// --- Node 1: Generate Outline ---
node! {
    pub struct OutlineNode {
        llm: LLMProviderWrapper,
    };
    context = ();
    input = String; // Input: Article Topic
    output = String; // Output: Outline Text
    | _ctx, topic | {
        println!("OutlineNode: Generating outline for topic: '{}'", topic);
        // Call LLM to generate an outline
        let prompt = format!("Generate an outline for an article about: {}", topic);
        let messages = vec![ChatMessage { role: ChatRole::User, content: prompt, message_type: MessageType::Text }];
        let outline: String = self.llm.inner.chat(&messages).await.map_err(|e| FloxideError::Generic(e.to_string()))?.text().unwrap_or_default();
        // Pass the outline to the next step
        Ok(Transition::Next(outline))
    }
}

// --- Node 2: Draft Article ---
node! {
    pub struct DraftNode {
        llm: LLMProviderWrapper,
    };
    context = ();
    input = String; // Input: Outline Text
    output = String; // Output: Draft Article Text
    | _ctx, outline | {
        println!("DraftNode: Drafting article based on outline...");
        // Call LLM to draft the article based on the outline
        let prompt = format!("Write a detailed draft article based on the following outline:\n{}", outline);
        let messages = vec![ChatMessage { role: ChatRole::User, content: prompt, message_type: MessageType::Text }];
        let draft: String = self.llm.inner.chat(&messages).await.map_err(|e| FloxideError::Generic(e.to_string()))?.text().unwrap_or_default();
        // Pass the draft to the next step
        Ok(Transition::Next(draft))
    }
}

// --- Node 3: Review Article ---
node! {
    pub struct ReviewNode {
        llm: LLMProviderWrapper,
    };
    context = ();
    input = String; // Input: Draft Article Text
    output = String; // Output: Final Article Text
    | _ctx, draft | {
        println!("ReviewNode: Reviewing and finalizing draft...");
        // Call LLM to review and finalize the draft
        let prompt = format!("Review and finalize the following article draft. Provide the final polished version without any additional text:\n{}", draft);
        let messages = vec![ChatMessage { role: ChatRole::User, content: prompt, message_type: MessageType::Text }];
        let final_article: String = self.llm.inner.chat(&messages).await.map_err(|e| FloxideError::Generic(e.to_string()))?.text().unwrap_or_default();
        // Pass the final article as the workflow result
        Ok(Transition::Next(final_article))
    }
}

// --- Workflow Definition: Connecting the nodes ---
workflow! {
    pub struct ArticleWriterWorkflow {
        outline: OutlineNode,
        draft: DraftNode,
        review: ReviewNode,
    }
    start = outline; // Start with the outline node
    context = ();
    edges {
        // Define the sequence: outline -> draft -> review
        outline => { [draft] };
        draft => { [review] };
        review => {}; // review is the final node
    }
}

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Initialize the LLM for chat completion
    let llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"))
        .model("gpt-4o")
        .temperature(0.7)
        .build()
        .expect("Failed to build LLM");

    let llm = LLMProviderWrapper::new(Arc::new(llm));

    let workflow = ArticleWriterWorkflow {
        outline: OutlineNode { llm: llm.clone() },
        draft: DraftNode { llm: llm.clone() },
        review: ReviewNode { llm: llm.clone() },
    };
    
    let ctx = WorkflowCtx::new(());
    let result = workflow
        .run(&ctx, "Rust Programming Language".to_string())
        .await?;

    println!("Generated article: {}", result);
    
    Ok(())
}
```
## Examples

The `examples/` directory contains various demonstrations of Floxide's features:

*   **`node_macro_example.rs`**: Basic usage of the `node!` macro to define a node with internal state and custom context.
*   **`branching_example.rs`**: Demonstrates a workflow with a shared context (`MyCtx`) and composite nodes that branch based on an enum output (`FooAction`).
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
*   **`order_example.rs`**: A workflow that simulates an order processing system, including validation, payment processing, and stock allocation.
*   **`llm_example.rs`**: A simple linear workflow that shows how to use LLM-like steps to generate an article.
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