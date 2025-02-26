# Installation

Getting started with Floxide is straightforward. This guide will walk you through the installation process and help you set up your first Floxide project.

## Prerequisites

Before installing Floxide, ensure you have the following prerequisites:

- **Rust and Cargo**: Floxide is a Rust library, so you'll need Rust and Cargo installed. If you don't have them installed, you can get them from [rustup.rs](https://rustup.rs/).
- **Rust version**: Floxide requires Rust 1.65 or later due to its use of async traits.

## Adding Floxide to Your Project

### Creating a New Project

If you're starting a new project, create a new Rust project using Cargo:

```bash
cargo new my_floxide_project
cd my_floxide_project
```

### Adding Dependencies

Add Floxide to your project by adding the following to your `Cargo.toml` file:

```toml
[dependencies]
floxide-core = "0.1.0"
tokio = { version = "1.28", features = ["full"] }
async-trait = "0.1.68"
```

- **floxide-core**: The core library containing the fundamental abstractions and workflow engine.
- **tokio**: The async runtime used by Floxide.
- **async-trait**: Required for using async functions in traits.

### Optional Crates

Depending on your needs, you might want to add additional Floxide crates:

```toml
[dependencies]
# For transform-based workflows
floxide-transform = "0.1.0"

# For batch processing
floxide-batch = "0.1.0"

# For event-driven workflows
floxide-event = "0.1.0"

# For time-based workflows
floxide-timer = "0.1.0"

# For reactive workflows
floxide-reactive = "0.1.0"

# For long-running workflows
floxide-longrunning = "0.1.0"
```

## Verifying Installation

To verify that Floxide is correctly installed, create a simple program that uses Floxide:

```rust
use floxide_core::{lifecycle_node, LifecycleNode, Workflow, DefaultAction};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct SimpleContext {
    message: String,
}

fn main() {
    println!("Floxide is installed correctly!");
}
```

Build your project to ensure all dependencies are resolved:

```bash
cargo build
```

If the build succeeds, you've successfully installed Floxide!

## Next Steps

Now that you have Floxide installed, you can:

- Continue to the [Quick Start Guide](quick-start.md) to create your first workflow
- Explore the [Core Concepts](../core-concepts/overview.md) to understand the fundamental abstractions
- Check out the [Examples](../examples/basic-workflow.md) to see Floxide in action
