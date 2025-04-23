// examples/node_macro_example.rs
// Demonstrates the `node!` macro: struct fields, custom context, input/output, and workflow usage.

use floxide_macros::{workflow, node};
use async_trait::async_trait;
use floxide_core::{transition::Transition, Node, WorkflowCtx};

#[derive(Debug, Clone)]
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
        Ok(Transition::Finish)
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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