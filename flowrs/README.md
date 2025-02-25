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
