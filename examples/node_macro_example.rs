// examples/node_macro_example.rs
// Demonstrates the `node!` macro: struct fields, custom context, input/output, and workflow usage.

use floxide_macros::{workflow, node};
use floxide_core::{transition::Transition, Node, WorkflowCtx};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyCtx;

// Define a multiplying node with a `factor` field
node! {
    pub struct MultiplierNode {
        factor: i32,
    };
    context = MyCtx;
    input = i32;
    output = i32;
    |ctx, x| {
        // You can inspect or modify the shared context if needed
        println!("MultiplierNode: ctx = {:?}", ctx);
        println!("MultiplierNode: {} * {} = {}", x, self.factor, x * self.factor);
        Ok(Transition::Next(x * self.factor))
    }
}

// Define a printing node that finishes the workflow
node! {
    pub struct PrinterNode {};
    context = MyCtx;
    input = i32;
    output = ();
    |ctx, x| {
        println!("PrinterNode: ctx = {:?}", ctx);
        println!("PrinterNode: final output = {}", x);
        Ok(Transition::Next(()))
    }
}

// Compose them into a simple workflow
workflow! {
    pub struct NodeMacroWorkflow {
        mult: MultiplierNode,
        print: PrinterNode,
    }
    context = MyCtx;
    start = mult;
    edges {
        mult => { [ print ] };
        print => {};
    }
}

/// Runs the NodeMacroWorkflow with input 3 and returns Ok(()) if it succeeds
pub async fn run_node_macro_example() -> Result<(), Box<dyn std::error::Error>> {
    // Instantiate the workflow with node instances
    let wf = NodeMacroWorkflow {
        mult: MultiplierNode { factor: 4 },
        print: PrinterNode {},
    };
    // Use unit context
    let ctx = WorkflowCtx::new(MyCtx);
    println!("Running node_macro_example with input 3:");
    wf.run(&ctx, 3).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_node_macro_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_node_macro_example() {
        run_node_macro_example().await.expect("workflow should run");
    }
}