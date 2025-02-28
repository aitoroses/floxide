# 🚀 Floxide: The Power of Workflows in Rust

[![CI](https://github.com/aitoroses/floxide/actions/workflows/ci.yml/badge.svg)](https://github.com/aitoroses/floxide/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/floxide-core.svg)](https://crates.io/crates/floxide-core)
[![Documentation](https://docs.rs/floxide-core/badge.svg)](https://docs.rs/floxide-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> A type-safe, composable directed graph workflow system written in Rust.

## 💫 Overview

Floxide transforms complex workflow orchestration into a delightful experience. Built with Rust's powerful type system at its core, Floxide provides a flexible, performant, and type-safe way to create sophisticated workflow graphs with crystal-clear transitions between steps.

## ✨ Key Features

- **🔒 Type-Safe By Design**: Leverage Rust's type system for compile-time workflow correctness
- **🧩 Composable Architecture**: Build complex workflows from simple, reusable components
- **⚡ Async First**: Native support for asynchronous execution with Tokio
- **🔄 Advanced Patterns**: Support for batch processing, event-driven workflows, and more
- **💾 State Management**: Built-in serialization for workflow persistence
- **🔍 Observability**: Comprehensive tracing and monitoring capabilities
- **🧪 Testable**: Design your workflows for easy testing and verification

## 📐 Architectural Decisions

Floxide is built on a foundation of carefully considered architectural decisions, each documented in our project documentation. These decisions form the backbone of our design philosophy and guide the evolution of the framework.

### Core Design Philosophy

At the heart of Floxide lies a commitment to **type safety**, **composability**, and **performance**. Our architecture embraces Rust's powerful type system to catch errors at compile time rather than runtime, while providing flexible abstractions that can be composed to create complex workflows.

Key architectural principles include:

- **Trait-based polymorphism** over inheritance
- **Ownership-aware design** that leverages Rust's borrowing system
- **Zero-cost abstractions** that compile to efficient code
- **Explicit error handling** with rich error types
- **Async-first approach** with Tokio integration

### Documentation Resources

For detailed information about Floxide's architecture and implementation:

- **[API Documentation](https://docs.rs/floxide/latest/floxide/)** - Comprehensive Rust API docs with detailed explanations of types, traits, and functions
- **[Project Documentation](https://aitoroses.github.io/floxide/)** - User guides, tutorials, and architectural overviews

Our architecture is not static—it evolves through a rigorous process of evaluation and refinement. Each new feature or enhancement is preceded by careful design consideration that documents the context, decision, and consequences, ensuring that our architectural integrity remains strong as the framework grows.

This approach has enabled us to build a framework that is both powerful and flexible, capable of handling everything from simple linear workflows to complex event-driven systems with parallel processing capabilities.

## 🚀 Quick Start

Add Floxide to your project:

```toml
[dependencies]
floxide = { version = "1.0.0", features = ["transform", "event"] }
```

Create your first workflow:

```rust
use floxide::{lifecycle_node, LifecycleNode, Workflow, DefaultAction, FloxideError};
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
        input: "Hello, Floxide!".to_string(),
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

Floxide uses feature flags to allow you to include only the functionality you need:

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
floxide = { version = "1.0.0", features = ["transform"] }

# Include event-driven and timer functionality
floxide = { version = "1.0.0", features = ["event", "timer"] }

# Include all functionality
floxide = { version = "1.0.0", features = ["full"] }
```

## 🧩 Workflow Pattern Examples

Floxide supports a wide variety of workflow patterns through its modular crate system. Each pattern is designed to solve specific workflow challenges:

### 🔄 Simple Chain (Linear Workflow)

A basic sequence of nodes executed one after another. This is the foundation of all workflows.

```mermaid
graph LR
    A["TextAnalysisNode"] --> B["UppercaseNode"]
    A -->|"prep → exec → post"| A
    B -->|"prep → exec → post"| B
    style A fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style B fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
```

**Example:** [lifecycle_node.rs](https://github.com/aitoroses/floxide/blob/main/examples/lifecycle_node.rs) - Demonstrates a workflow with preparation, execution, and post-processing phases.

### 🌲 Conditional Branching

Workflows that make decisions based on context data or node results, directing flow through different paths.

```mermaid
graph TD
    A["ValidateOrderNode"] -->|Valid| B["ProcessPaymentNode"]
    A -->|Invalid| E["CancelOrderNode"]
    B -->|Success| C["ShipOrderNode"]
    B -->|Error| F["NotificationNode"]
    C --> D["DeliverOrderNode"]
    F --> B
    style A fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style B fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style C fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style D fill:#c4e6ff,stroke:#1a73e8,stroke-width:2px,color:black
    style E fill:#ffcccc,stroke:#e53935,stroke-width:2px,color:black
    style F fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
```

**Example:** [order_processing.rs](https://github.com/aitoroses/floxide/blob/main/examples/order_processing.rs) - Implements a complete order processing workflow with validation, payment, shipping, and error handling.

### 🔄 Transform Pipeline

A specialized workflow for data transformation, where each node transforms input to output in a functional style.

```mermaid
graph LR
    A["Input"] --> B["TextTransformer"]
    B --> C["TextAnalyzer"]
    C --> D["GreetingTransformer"]
    B -->|"prep → exec → post"| B
    C -->|"prep → exec → post"| C
    D -->|"prep → exec → post"| D
    style A fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style B fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style C fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
    style D fill:#e8f5e9,stroke:#43a047,stroke-width:2px,color:black
```

**Examples:** 
- [transform_node.rs](https://github.com/aitoroses/floxide/blob/main/examples/transform_node.rs) - Basic implementation of the Transform Node pattern
- [transform_node.rs](https://github.com/aitoroses/floxide/blob/main/examples/transform_node.rs) - Advanced examples of transform nodes with multiple implementation approaches

### 🔀 Parallel Batch Processing

Process multiple items concurrently with controlled parallelism, ideal for high-throughput data processing.

```mermaid
graph TD
    A["ImageBatchContext"] --> B["Split Batch"]
    B --> C1["SimpleImageProcessor<br>(Image 1)"]
    B --> C2["SimpleImageProcessor<br>(Image 2)"]
    B --> C3["SimpleImageProcessor<br>(Image 3)"]
    C1 --> D["Update Batch Context"]
    C2 --> D
    C3 --> D
    D -->|"Statistics"| E["Print Results"]
    style A fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style B fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style C1 fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style C2 fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style C3 fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style D fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
    style E fill:#e3f2fd,stroke:#1565c0,stroke-width:2px,color:black
```

**Example:** [batch_processing.rs](https://github.com/aitoroses/floxide/blob/main/examples/batch_processing.rs) - Demonstrates parallel processing of images with controlled concurrency and resource management.

### 📡 Event-Driven Flow

Workflows that respond to external events, ideal for building reactive systems that process events as they arrive.

```mermaid
graph TD
    A["TemperatureEventSource"] -->|"Events"| B["TemperatureClassifier"]
    B -->|"Normal"| C["NormalTempHandler"]
    B -->|"High"| D["HighTempHandler"]
    B -->|"Low"| E["LowTempHandler"]
    B -->|"Critical"| F["CriticalTempHandler"]
    C --> A
    D --> A
    E --> A
    F -->|"Terminate"| G["End Workflow"]
    style A fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style B fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style C fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style D fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style E fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style F fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
    style G fill:#e8eaf6,stroke:#3949ab,stroke-width:2px,color:black
```

**Example:** [event_driven_workflow.rs](https://github.com/aitoroses/floxide/blob/main/examples/event_driven_workflow.rs) - Implements a temperature monitoring system that processes events from multiple sensors and triggers appropriate actions.

### ⏱️ Time-Based Workflows

Workflows that execute based on time schedules, supporting one-time, interval, and calendar-based scheduling.

```mermaid
graph TD
    A["SimpleTimer<br>(Once)"] -->|"Trigger"| B["CounterContext"]
    C["IntervalTimer<br>(Every 5s)"] -->|"Trigger"| B
    D["CronTimer<br>(Schedule)"] -->|"Trigger"| B
    E["DailyTimer<br>(10:00 AM)"] -->|"Trigger"| B
    F["WeeklyTimer<br>(Monday)"] -->|"Trigger"| B
    style A fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style B fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style C fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style D fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style E fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
    style F fill:#fff8e1,stroke:#ff8f00,stroke-width:2px,color:black
```

**Example:** [timer_node.rs](https://github.com/aitoroses/floxide/blob/main/examples/timer_node.rs) - Shows how to create and use timer nodes with different scheduling options (one-time, interval, and cron).

### 🔄 Reactive Workflows

Workflows that react to changes in external data sources, such as files, databases, or streams.

```mermaid
graph TD
    A["FileWatcherNode"] -->|"File Changed"| B["FileWatchContext"]
    C["SensorDataNode"] -->|"New Measurement"| D["SensorContext"]
    B -->|"Record Change"| E["Process Change"]
    D -->|"Record Measurement"| F["Process Measurement"]
    E --> A
    F --> C
    style A fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style B fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style C fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style D fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style E fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
    style F fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px,color:black
```

**Example:** [reactive_node.rs](https://github.com/aitoroses/floxide/blob/main/examples/reactive_node.rs) - Demonstrates how to build reactive nodes that respond to changes in data sources and trigger appropriate actions.

### ⏸️ Long-Running Processes

Workflows for processes that can be suspended and resumed, with state persistence between executions.

```mermaid
graph TD
    A["Start Process"] --> B["SimpleLongRunningNode"]
    B -->|"Execute Step"| C["Checkpoint"]
    C -->|"Complete"| D["Final Result"]
    C -->|"Suspend"| E["Save State"]
    E -->|"Resume"| B
    style A fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style B fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style C fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style D fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
    style E fill:#fce4ec,stroke:#c2185b,stroke-width:2px,color:black
```

**Example:** [longrunning_node.rs](https://github.com/aitoroses/floxide/blob/main/examples/longrunning_node.rs) - Shows how to implement long-running processes with checkpointing and resumption capabilities.

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

- [Complete API Documentation](https://docs.rs/floxide-core)
- [Example Workflows](https://github.com/aitoroses/floxide/tree/main/examples)
- [Architectural Decision Records](https://github.com/aitoroses/floxide/tree/main/docs/adrs)

Try our examples directly:

```bash
git clone https://github.com/aitoroses/floxide.git
cd floxide
cargo run --example lifecycle_node
```

## 🤝 Contributing

We welcome contributions of all kinds! Whether you're fixing a bug, adding a feature, or improving documentation, your help is appreciated.

See our [Contributing Guidelines](CONTRIBUTING.md) for more details on how to get started.

## 📄 License

Floxide is available under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- The Rust community for their excellent crates and support
- Our amazing contributors who help make Floxide better every day
<!-- Trigger rebuild: martes, 25 de febrero de 2025, 18:49:33 CET -->
