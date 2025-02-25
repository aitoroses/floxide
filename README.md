# 🚀 Flowrs: The Power of Workflows in Rust

[![CI](https://github.com/aitoroses/flowrs/actions/workflows/ci.yml/badge.svg)](https://github.com/aitoroses/flowrs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/flowrs-core.svg)](https://crates.io/crates/flowrs-core)
[![Documentation](https://docs.rs/flowrs-core/badge.svg)](https://docs.rs/flowrs-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> A type-safe, composable directed graph workflow system written in Rust.

## 💫 Overview

Flowrs transforms complex workflow orchestration into a delightful experience. Built with Rust's powerful type system at its core, Flowrs provides a flexible, performant, and type-safe way to create sophisticated workflow graphs with crystal-clear transitions between steps.

## ✨ Key Features

- **🔒 Type-Safe By Design**: Leverage Rust's type system for compile-time workflow correctness
- **🧩 Composable Architecture**: Build complex workflows from simple, reusable components
- **⚡ Async First**: Native support for asynchronous execution with Tokio
- **🔄 Advanced Patterns**: Support for batch processing, event-driven workflows, and more
- **💾 State Management**: Built-in serialization for workflow persistence
- **🔍 Observability**: Comprehensive tracing and monitoring capabilities
- **🧪 Testable**: Design your workflows for easy testing and verification

## 📐 Architectural Decisions

This project follows documented architectural decisions recorded in ADRs (Architectural Decision Records). Each ADR captures the context, decision, and consequences of significant architectural choices. The development is guided by an LLM with rules defined in the `.cursorrules` file.

Key architectural decisions include:

- [**Core Framework Abstractions**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0003-core-framework-abstractions.md) - Defining the fundamental abstractions like Node, Action, and Workflow with a trait-based approach for type safety and flexibility.

- [**Project Structure and Crate Organization**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0002-project-structure-and-crate-organization.md) - Organizing the framework as a Cargo workspace with multiple specialized crates for modularity and separation of concerns.

- [**Async Runtime Selection**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0004-async-runtime-selection.md) - Choosing Tokio as the primary async runtime for its comprehensive feature set and wide adoption.

- [**Node Lifecycle Methods**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0008-node-lifecycle-methods.md) - Implementing a three-phase lifecycle (prep/exec/post) for workflow nodes to provide clear separation of concerns.

- [**Batch Processing Implementation**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0007-batch-processing-implementation.md) - Designing a batch processing system that efficiently handles parallel execution with configurable concurrency limits.

- [**Event-Driven Workflow Pattern**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0009-event-driven-workflow-pattern.md) - Extending the framework with event-driven capabilities for handling asynchronous events.

- [**Reactive Node Implementation**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0017-reactive-node-implementation.md) - Creating nodes that can respond to changes in external data sources using a stream-based approach.

- [**Timer Node Implementation**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0021-timer-node-implementation.md) - Supporting time-based scheduling for workflow execution with various scheduling patterns.

- [**Long-Running Node Implementation**](https://github.com/aitoroses/flowrs/tree/main/docs/adrs/0022-longrunning-node-implementation.md) - Enabling workflows to process work incrementally with state persistence between executions.

## 🚀 Quick Start

Add Flowrs to your project:

```toml
[dependencies]
flowrs = { version = "1.0.0", features = ["transform", "event"] }
```

Create your first workflow:

```rust
use flowrs::{lifecycle_node, LifecycleNode, Workflow, DefaultAction, FlowrsError};
use async_trait::async_trait;
use std::sync::Arc;

// Define your context type
#[derive(Debug, Clone)]
struct MessageContext {
    input: String,
    result: Option<String>,
}

// Create a node using the convenience function
fn create_processor_node() -> impl LifecycleNode<MessageContext, DefaultAction> {
    lifecycle_node(
        Some("processor"), // Node ID
        |ctx: &mut MessageContext| async move {
            // Preparation phase
            println!("Preparing to process: {}", ctx.input);
            Ok(ctx.input.clone())
        },
        |input: String| async move {
            // Execution phase
            println!("Processing message...");
            Ok(format!("✅ Processed: {}", input))
        },
        |_prep, exec_result, ctx: &mut MessageContext| async move {
            // Post-processing phase
            ctx.result = Some(exec_result);
            Ok(DefaultAction::Next)
        },
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a context
    let mut context = MessageContext {
        input: "Hello, Flowrs!".to_string(),
        result: None,
    };

    // Create a node and workflow
    let node = Arc::new(create_processor_node());
    let mut workflow = Workflow::new(node);

    // Execute the workflow
    workflow.execute(&mut context).await?;

    // Print the result
    println!("Result: {:?}", context.result);

    Ok(())
}
```

## 📦 Feature Flags

Flowrs uses feature flags to allow you to include only the functionality you need:

| Feature | Description | Dependencies |
|---------|-------------|-------------|
| `core` | Core abstractions and functionality (default) | None |
| `transform` | Transform node implementations | `core` |
| `event` | Event-driven workflow functionality | `core` |
| `timer` | Time-based workflow functionality | `core` |
| `longrunning` | Long-running process functionality | `core` |
| `reactive` | Reactive workflow functionality | `core` |
| `full` | All features | All of the above |

Example of using specific features:

```toml
# Only include core and transform functionality
flowrs = { version = "1.0.0", features = ["transform"] }

# Include event-driven and timer functionality
flowrs = { version = "1.0.0", features = ["event", "timer"] }

# Include all functionality
flowrs = { version = "1.0.0", features = ["full"] }
```

## 🧩 Workflow Pattern Examples

Flowrs supports a wide variety of workflow patterns through its modular crate system. Each pattern is designed to solve specific workflow challenges:

### 🔄 Simple Chain (Linear Workflow)

A basic sequence of nodes executed one after another. This is the foundation of all workflows.

```mermaid
graph LR
    A["Process Data"] --> B["Format Output"] --> C["Store Result"]
    style A fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style B fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style C fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
```

**Example:** [lifecycle_node.rs](https://github.com/aitoroses/flowrs/tree/main/examples/lifecycle_node.rs)

### 🌲 Conditional Branching

Workflows that make decisions based on context data or node results, directing flow through different paths.

```mermaid
graph TD
    A["Validate Input"] -->|Valid| B["Process Data"]
    A -->|Invalid| C["Error Handler"]
    B -->|Success| D["Format Output"]
    B -->|Error| C
    style A fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style B fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style C fill:#ffcccc,stroke:#e53935,stroke-width:2px,color:black
    style D fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
```

**Example:** [order_processing.rs](https://github.com/aitoroses/flowrs/tree/main/examples/order_processing.rs)

### 🔄 Transform Pipeline

A specialized workflow for data transformation, where each node transforms input to output in a functional style.

```mermaid
graph LR
    A["Raw Data"] --> B["Validate"] --> C["Transform"] --> D["Format"] --> E["Output"]
    style A fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style B fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style C fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style D fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style E fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
```

**Example:** [transform_node.rs](https://github.com/aitoroses/flowrs/tree/main/examples/transform_node.rs)

### 🔀 Parallel Batch Processing

Process multiple items concurrently with controlled parallelism, ideal for high-throughput data processing.

```mermaid
graph TD
    A["Batch Input"] --> B["Split Batch"]
    B --> C1["Process Item 1"]
    B --> C2["Process Item 2"]
    B --> C3["Process Item 3"]
    C1 --> D["Aggregate Results"]
    C2 --> D
    C3 --> D
    style A fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style B fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style C1 fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style C2 fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style C3 fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style D fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
```

**Example:** [batch_processing.rs](https://github.com/aitoroses/flowrs/tree/main/examples/batch_processing.rs)

### 📡 Event-Driven Flow

Workflows that respond to external events, ideal for building reactive systems that process events as they arrive.

```mermaid
graph TD
    A["Event Source"] -->|Events| B["Event Classifier"]
    B -->|Type A| C["Handler A"]
    B -->|Type B| D["Handler B"]
    B -->|Type C| E["Handler C"]
    C --> F["Event Source"]
    D --> F
    E --> F
    style A fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style B fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style C fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style D fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style E fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style F fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
```

**Example:** [event_driven_workflow.rs](https://github.com/aitoroses/flowrs/tree/main/examples/event_driven_workflow.rs)

### ⏱️ Time-Based Workflows

Workflows that execute based on time schedules, supporting one-time, interval, and calendar-based scheduling.

```mermaid
graph TD
    A["Timer Source"] -->|Trigger| B["Scheduled Task"]
    B --> C["Process Result"]
    C -->|Reschedule| A
    style A fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style B fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style C fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
```

**Example:** [timer_node.rs](https://github.com/aitoroses/flowrs/tree/main/examples/timer_node.rs)

### 🔄 Reactive Workflows

Workflows that react to changes in external data sources, such as files, databases, or streams.

```mermaid
graph TD
    A["Data Source"] -->|Change| B["Change Detector"]
    B --> C["Process Change"]
    C --> D["Update State"]
    D --> A
    style A fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style B fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style C fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style D fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
```

**Example:** [reactive_node.rs](https://github.com/aitoroses/flowrs/tree/main/examples/reactive_node.rs)

### ⏸️ Long-Running Processes

Workflows for processes that can be suspended and resumed, with state persistence between executions.

```mermaid
graph TD
    A["Start Process"] --> B["Execute Step"]
    B -->|Complete| C["Final Result"]
    B -->|Suspend| D["Save State"]
    D -->|Resume| B
    style A fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style B fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style C fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style D fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
```

**Example:** [longrunning_node.rs](https://github.com/aitoroses/flowrs/tree/main/examples/longrunning_node.rs)

### 🤖 Multi-Agent LLM System

A workflow pattern for orchestrating multiple AI agents that collaborate to solve complex tasks.

```mermaid
graph TD
    A["User Input"] --> B["Router Agent"]
    B -->|Research Task| C["Research Agent"]
    B -->|Code Task| D["Coding Agent"]
    B -->|Analysis Task| E["Analysis Agent"]
    C --> F["Aggregator Agent"]
    D --> F
    E --> F
    F --> G["Response Generator"]
    G --> H["User Output"]
    style A fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style B fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style C fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style D fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style E fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style F fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style G fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
    style H fill:#e0f7fa,stroke:#00838f,stroke-width:2px,color:black
```

This pattern demonstrates how to build a multi-agent LLM system where specialized agents handle different aspects of a task. Each agent is implemented as a node in the workflow, with the router determining which agents to invoke based on the task requirements. The aggregator combines results from multiple agents before generating the final response.

**Implementation Example:**

```rust
// Define agent context
#[derive(Debug, Clone)]
struct AgentContext {
    user_query: String,
    agent_responses: HashMap<String, String>,
    final_response: Option<String>,
}

// Create router agent node
fn create_router_agent() -> impl LifecycleNode<AgentContext, AgentAction> {
    lifecycle_node(
        Some("router"),
        |ctx: &mut AgentContext| async move {
            // Preparation: analyze the query
            println!("Router analyzing query: {}", ctx.user_query);
            Ok(ctx.user_query.clone())
        },
        |query: String| async move {
            // Execution: determine which agents to invoke
            let requires_research = query.contains("research") || query.contains("information");
            let requires_coding = query.contains("code") || query.contains("program");
            let requires_analysis = query.contains("analyze") || query.contains("evaluate");

            Ok((requires_research, requires_coding, requires_analysis))
        },
        |_prep, (research, coding, analysis), ctx: &mut AgentContext| async move {
            // Post-processing: route to appropriate agents
            if research {
                return Ok(AgentAction::Research);
            } else if coding {
                return Ok(AgentAction::Code);
            } else if analysis {
                return Ok(AgentAction::Analyze);
            }
            Ok(AgentAction::Aggregate) // Default if no specific routing
        },
    )
}

// Similar implementations for research_agent, coding_agent, analysis_agent, and aggregator_agent
```

## 📚 Examples & Documentation

Explore our extensive examples and documentation:

- [Complete API Documentation](https://docs.rs/flowrs-core)
- [Example Workflows](https://github.com/aitoroses/flowrs/tree/main/examples)
- [Architectural Decision Records](https://github.com/aitoroses/flowrs/tree/main/docs/adrs)

Try our examples directly:

```bash
git clone https://github.com/aitoroses/flowrs.git
cd flowrs
cargo run --example lifecycle_node
```

## 🤝 Contributing

We welcome contributions of all kinds! Whether you're fixing a bug, adding a feature, or improving documentation, your help is appreciated.

See our [Contributing Guidelines](CONTRIBUTING.md) for more details on how to get started.

## 📄 License

Flowrs is available under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- The Rust community for their excellent crates and support
- Our amazing contributors who help make Flowrs better every day
<!-- Trigger rebuild: martes, 25 de febrero de 2025, 18:49:33 CET -->
