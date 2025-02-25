# Installation

Getting started with Flowrs is straightforward. This guide will walk you through the installation process and help you set up your first Flowrs project.

## Prerequisites

Before installing Flowrs, ensure you have the following prerequisites:

- **Rust and Cargo**: Flowrs is a Rust library, so you'll need Rust and Cargo installed. If you don't have them installed, you can get them from [rustup.rs](https://rustup.rs/).
- **Rust version**: Flowrs requires Rust 1.65 or later due to its use of async traits.

## Adding Flowrs to Your Project

### Creating a New Project

If you're starting a new project, create a new Rust project using Cargo:

```bash
cargo new my_flowrs_project
cd my_flowrs_project
```

### Adding Dependencies

Add Flowrs to your project by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
flowrs-core = "0.1.0"
tokio = { version = "1.28", features = ["full"] }
async-trait = "0.1.68"
```

- **flowrs-core**: The core library containing the fundamental abstractions and workflow engine.
- **tokio**: The async runtime used by Flowrs.
- **async-trait**: Required for using async functions in traits.

### Optional Crates

Depending on your needs, you might want to add additional Flowrs crates:

```toml
[dependencies]
# For transform-based workflows
flowrs-transform = "0.1.0"

# For batch processing
flowrs-batch = "0.1.0"

# For event-driven workflows
flowrs-event = "0.1.0"

# For time-based workflows
flowrs-timer = "0.1.0"

# For reactive workflows
flowrs-reactive = "0.1.0"

# For long-running workflows
flowrs-longrunning = "0.1.0"
```

## Verifying Installation

To verify that Flowrs is correctly installed, create a simple program that uses Flowrs:

```rust
use flowrs_core::{lifecycle_node, LifecycleNode, Workflow, DefaultAction};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct SimpleContext {
    message: String,
}

fn main() {
    println!("Flowrs is installed correctly!");
}
```

Build your project to ensure all dependencies are resolved:

```bash
cargo build
```

If the build succeeds, you've successfully installed Flowrs!

## Next Steps

Now that you have Flowrs installed, you can:

- Continue to the [Quick Start Guide](quick-start.md) to create your first workflow
- Explore the [Core Concepts](../core-concepts/overview.md) to understand the fundamental abstractions
- Check out the [Examples](../examples/basic-workflow.md) to see Flowrs in action
