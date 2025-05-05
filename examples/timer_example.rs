// examples/timer_example.rs
// Demonstrates a simple timer-driven workflow using a channel source node,
// a mapping node, and a printing node.

use floxide::{workflow, WorkflowCtx};
// use async_trait::async_trait;
use floxide::node;
use floxide::{error::FloxideError, source, transition::Transition, Node};
use tokio::time::{sleep, Duration};

// A node that doubles its input.
node! {
    pub struct DoubleNode {};
    context = ();
    input = u64;
    output = u64;
    |ctx, x| {
        println!("DoubleNode: ctx = {:?}", ctx);
        println!("DoubleNode: input = {}", x);
        Ok(Transition::Next(x * 2))
    }
}

// A node that prints incoming values.
node! {
    pub struct PrintNode {};
    context = ();
    input = u64;
    output = ();
    |ctx, x| {
        println!("PrintNode: ctx = {:?}", ctx);
        println!("PrintNode: input = {}", x);
        Ok(Transition::Next(()))
    }
}

// A workflow that doubles then prints each value.
workflow! {
    pub struct PrintDouble {
        doubler: DoubleNode,
        printer:  PrintNode,
    }
    context = ();
    start = doubler;
    edges {
        doubler => {[printer]};
        printer => {};
    }
}

/// Runs the timer workflow and returns Ok(()) if it succeeds
pub async fn run_timer_example() -> Result<(), FloxideError> {
    // Create a channel-backed source for u64 ticks, capacity 10.
    let (tx, source) = source::<(), u64>(10);

    // Spawn a task that sends ticks 0..=9 at 1-second intervals, then closes
    tokio::spawn(async move {
        for i in 0u64..10u64 {
            sleep(Duration::from_millis(200)).await;
            if tx.send(i).await.is_err() {
                break;
            }
        }
        // Sender dropped here; receiver will see EOF
    });

    // Create the workflow instance and context.
    let ctx = WorkflowCtx::new(());
    let wf = PrintDouble {
        doubler: DoubleNode {},
        printer: PrintNode {},
    };
    // Drive the workflow for each incoming tick until source closes.
    source.run(&wf, &ctx).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_timer_example().await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_timer_example() {
        run_timer_example()
            .await
            .expect("timer workflow should run");
    }
}
