# Flowrs Framework

A directed graph workflow system written in Rust.

## Overview

Flowrs is a robust framework for building and executing workflow graphs. It provides a flexible and type-safe way to create complex workflows with clearly defined transitions between steps.

## Features

- Type-safe workflow definitions
- Composition-based node system
- Async execution of workflow steps
- Built-in retry mechanisms
- Batch processing and parallel execution
- Workflow state serialization
- Comprehensive observability through OpenTelemetry

## Workflow Patterns

Flowrs supports a variety of workflow patterns to handle different processing needs:

### Node

A single step operation that processes input and produces output. In this example, a node that summarizes an email as a standalone operation.

```mermaid
graph LR
    A["Summarize Email"] --> B[Output]
    style A fill:#f9f,stroke:#333,stroke-width:2px
```

### Chain

A sequence of connected nodes where the output of one node becomes the input to the next. Here, we first summarize an email and then draft a reply based on that summary.

```mermaid
graph LR
    A["Summarize Email"] --> B["Draft Reply"]
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
```

### Batch

Repeats the same processing step across multiple inputs in parallel. This pattern allows us to summarize multiple emails simultaneously, improving throughput for repetitive tasks.

```mermaid
graph LR
    subgraph "Batch Processing"
    A1["Summarize Email"]
    A2["Summarize Email"]
    A3["Summarize Email"]
    end
    Input --> A1 & A2 & A3
    style A1 fill:#f9f,stroke:#333,stroke-width:2px
    style A2 fill:#f9f,stroke:#333,stroke-width:2px
    style A3 fill:#f9f,stroke:#333,stroke-width:2px
```

### Async

Handles operations that involve waiting for I/O or external events. In this example, we check an inbox (which involves I/O wait) and then process new emails when they arrive.

```mermaid
graph LR
    A["Check Inbox"] -->|New Email| B["Summarize Email"]
    A -->|No New Email| A
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
```

### Shared

Enables communication between nodes through shared state. Here, the "Summarize Email" node writes an email summary to a shared state, and the "Draft Reply" node reads from that shared state rather than receiving direct input from the previous node.

```mermaid
graph LR
    A["Summarize Email"] -->|write| S[("Shared State: Email Summary")]
    S -->|read| B["Draft Reply"]
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
    style S fill:#bbf,stroke:#333,stroke-width:2px
```

### Branch

Implements conditional logic to determine the next step based on certain criteria. In this workflow, after summarizing an email, we determine if it needs review. If it does, it goes to the "Review" node and then to "Draft Reply" after approval. If review is not needed, it goes directly to "Draft Reply".

```mermaid
graph TD
    A["Summarize Email"] -->|Need Review| B["Review"]
    A -->|Approved| C["Draft Reply"]
    B -->|Approved| C
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#f9f,stroke:#333,stroke-width:2px
```

### Nesting

Allows workflows to be composed of other workflows, creating reusable components. In this example, we have a "Coding Task" node that triggers a nested workflow for software development. This nested workflow includes writing tests, writing code, verifying the code, and analyzing its complexity.

```mermaid
graph LR
    A["Coding Task"] --> B
    subgraph "Development Workflow"
    B["Write Tests"] --> C["Write Code"] --> D["Verify Code"] --> E["Analyze Complexity"]
    end
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#f9f,stroke:#333,stroke-width:2px
    style D fill:#f9f,stroke:#333,stroke-width:2px
    style E fill:#f9f,stroke:#333,stroke-width:2px
```

### Looping

Implements repetitive processes that continue until a condition is met. This long-running workflow starts with "Get Question", proceeds to "Answer Question", and then loops back to "Get Question" to continue the cycle indefinitely.

```mermaid
graph LR
    A["Get Question"] --> B["Answer Question"]
    B --> A
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
```

## Workflow Composition

The true power of Flowrs comes from combining these patterns to create complex, real-world workflows. Each pattern addresses a specific workflow need, and they can be composed to solve sophisticated business problems:

### Pattern Combinations

- **Chain + Branch**: Create sequential workflows with decision points
- **Batch + Async**: Process multiple items in parallel while handling I/O operations
- **Nesting + Shared**: Build reusable workflow components that communicate through shared state
- **Looping + Branch**: Implement iterative processes with exit conditions

### Composition Benefits

1. **Modularity**: Break complex workflows into manageable, reusable components
2. **Flexibility**: Adapt workflows to changing requirements by recombining patterns
3. **Maintainability**: Update specific parts of a workflow without affecting the whole
4. **Scalability**: Handle increasing workloads by applying batch processing to appropriate steps

### Visual Workflow Design

When designing workflows with Flowrs, consider visualizing them first using the patterns shown above. This helps identify:

- Which steps can be processed in parallel (Batch)
- Where conditional logic is needed (Branch)
- Which components can be reused (Nesting)
- Where shared state is required (Shared)
- Which operations need to wait for external events (Async)

### Example: Email Processing Workflow

Here's an example of a complex email processing workflow that combines multiple patterns:

```mermaid
graph TD
    Start[Start] --> CheckInbox["Check Inbox (Async)"]
    CheckInbox --> HasEmails{Has New Emails?}
    HasEmails -->|No| Wait["Wait (Async)"]
    Wait --> CheckInbox
    HasEmails -->|Yes| BatchProcess["Process Emails (Batch)"]

    subgraph "For Each Email"
        BatchProcess --> Summarize["Summarize Email"]
        Summarize --> NeedsReview{Needs Review?}
        NeedsReview -->|Yes| Review["Review Email"]
        NeedsReview -->|No| Draft["Draft Reply"]
        Review --> Draft

        subgraph "Draft Reply Process (Nested)"
            Draft --> GenerateTemplate["Generate Template"]
            GenerateTemplate --> AddPersonalization["Add Personalization"]
            AddPersonalization --> CheckGrammar["Check Grammar"]
        end

        CheckGrammar --> SendReply["Send Reply"]
    end

    SendReply --> UpdateStatus["Update Status (Shared State)"]
    UpdateStatus --> CheckInbox

    style CheckInbox fill:#f9f,stroke:#333,stroke-width:2px
    style BatchProcess fill:#f9f,stroke:#333,stroke-width:2px
    style Summarize fill:#f9f,stroke:#333,stroke-width:2px
    style Review fill:#f9f,stroke:#333,stroke-width:2px
    style Draft fill:#f9f,stroke:#333,stroke-width:2px
    style GenerateTemplate fill:#f9f,stroke:#333,stroke-width:2px
    style AddPersonalization fill:#f9f,stroke:#333,stroke-width:2px
    style CheckGrammar fill:#f9f,stroke:#333,stroke-width:2px
    style SendReply fill:#f9f,stroke:#333,stroke-width:2px
    style UpdateStatus fill:#f9f,stroke:#333,stroke-width:2px
```

This workflow combines:

- **Async**: For checking the inbox and waiting
- **Batch**: For processing multiple emails in parallel
- **Branch**: For deciding whether emails need review
- **Chain**: For sequential processing steps
- **Nesting**: For the draft reply process
- **Shared**: For updating status across the workflow
- **Looping**: For continuously checking the inbox

## Paradigm Patterns

In addition to the core workflow patterns, Flowrs can be used to implement specialized workflow types for AI and automation applications:

### Workflow (Directed Path)

Simple directed path workflow with sequential processing.

```mermaid
graph LR
    A["Summarize Email"] --> B["Draft Reply"]
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
```

### Chat (Loop + Chat History Store)

Looping conversation flow with state management for chat history.

```mermaid
graph TD
    A["Chat"] --> A
    A -->|write| S[("Chat History")]
    S -->|read| A
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style S fill:#bbf,stroke:#333,stroke-width:2px
```

### RAG (Vector DB Store)

Workflows that integrate document storage and retrieval for question answering.

```mermaid
graph LR
    A["Upload Documents"] -->|write| S[("Vector DB")]
    S -->|read| B["Answer Questions"]
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
    style S fill:#bbf,stroke:#333,stroke-width:2px
```

### Chain of Thought (Loop + Think History Store)

Single "thinking" step that loops and maintains reasoning history.

```mermaid
graph TD
    A["Think"] --> A
    A -->|write| S[("Think History")]
    S -->|read| A
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style S fill:#bbf,stroke:#333,stroke-width:2px
```

### Map-Reduce (Batch + Merge)

Batch processing of data chunks followed by aggregation of results.

```mermaid
graph TD
    A["Map Chunks"] --> B1["Summarize Chunk"] & B2["Summarize Chunk"] & B3["Summarize Chunk"]
    B1 & B2 & B3 --> C["Reduce Summaries"]
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B1 fill:#f9f,stroke:#333,stroke-width:2px
    style B2 fill:#f9f,stroke:#333,stroke-width:2px
    style B3 fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#f9f,stroke:#333,stroke-width:2px
```

### Agent (Loop + Branching)

Autonomous workflows with branching decision logic and feedback loops.

```mermaid
graph TD
    A["Summarize Email"] --> B{Need Review?}
    B -->|Need review| C["Review"]
    B -->|Approved| D["Draft Reply"]
    C -->|Approved| D
    D --> A
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#f9f,stroke:#333,stroke-width:2px
    style D fill:#f9f,stroke:#333,stroke-width:2px
```

### Multi-Agent (Loop + Branching + Pub/Sub)

Complex interconnected workflows with multiple processing nodes and publish/subscribe communication.

```mermaid
graph TD
    A1 --> B1 & B2
    B1 --> C1 & C2
    B2 --> C2 & C3
    C1 & C2 & C3 -->|publish| S[("Pub/Sub")]
    S -->|subscribe| A1 & A2
    A2 --> B3 --> C4
    style A1 fill:#f9f,stroke:#333,stroke-width:2px
    style A2 fill:#f9f,stroke:#333,stroke-width:2px
    style B1 fill:#f9f,stroke:#333,stroke-width:2px
    style B2 fill:#f9f,stroke:#333,stroke-width:2px
    style B3 fill:#f9f,stroke:#333,stroke-width:2px
    style C1 fill:#f9f,stroke:#333,stroke-width:2px
    style C2 fill:#f9f,stroke:#333,stroke-width:2px
    style C3 fill:#f9f,stroke:#333,stroke-width:2px
    style C4 fill:#f9f,stroke:#333,stroke-width:2px
    style S fill:#bbf,stroke:#333,stroke-width:2px
```

### Supervisor (Nesting)

Nested workflows with oversight that can approve or reject work.

```mermaid
graph TD
    subgraph "Nested Workflow"
    A --> B --> C
    end
    Nested Workflow --> D["Supervise"]
    D -->|reject| Nested Workflow
    D -->|approve| E
    style A fill:#f9f,stroke:#333,stroke-width:2px
    style B fill:#f9f,stroke:#333,stroke-width:2px
    style C fill:#f9f,stroke:#333,stroke-width:2px
    style D fill:#f9f,stroke:#333,stroke-width:2px
    style E fill:#f9f,stroke:#333,stroke-width:2px
```

## Getting Started

Add flowrs to your Cargo.toml:

```toml
[dependencies]
flowrs-core = "0.1.0"
flowrs-async = "0.1.0"
```

### Example Usage

```rust
use flowrs_core::{Node, NodeOutcome, Workflow, ActionType, DefaultAction};

// Define a simple workflow node
struct GreetingNode;

impl Node<String, DefaultAction> for GreetingNode {
    type Output = String;

    async fn process(&self, name: &mut String) -> Result<NodeOutcome<Self::Output, DefaultAction>, FlowrsError> {
        let greeting = format!("Hello, {}!", name);
        Ok(NodeOutcome::Complete(greeting))
    }
}

// Build and run a workflow
async fn run_workflow() {
    let mut workflow = Workflow::new(GreetingNode);
    let mut name = String::from("World");

    let result = workflow.execute(&mut name).await;
    assert_eq!(result.unwrap(), "Hello, World!");
}
```

## Project Structure

The framework is organized as a Cargo workspace with multiple crates:

- `flowrs-core`: Core traits and structures for the framework
- `flowrs-async`: Async runtime integration
- `examples`: Example workflow implementations
- `benches`: Performance benchmarks

## Documentation

For detailed documentation, see:

- [API Documentation](https://docs.rs/flowrs-core) (coming soon)
- [User Guide](docs/guide/README.md) (coming soon)
- [Architectural Decision Records](docs/adrs/README.md)

## License

This project is licensed under the MIT License - see the LICENSE file for details.
