// examples/minimal_example.rs
use floxide_core::*;
use async_trait::async_trait;
use std::collections::VecDeque;

/// Action enum for FooNode: branch carries different payloads
#[derive(Clone, Debug)]
pub enum FooAction {
    Above(u64),    // above threshold: carries numeric value
    Below(String), // below threshold: carries descriptive message
}

/// Node that compares input against a threshold
#[derive(Clone, Debug)]
pub struct FooNode {
    threshold: u64,
}

#[async_trait]
impl Node for FooNode {
    type Input = u64;
    type Output = FooAction;

    async fn process<C>(
        &self,
        _ctx: &C,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        if input > self.threshold {
            let out = input * 2;
            println!("FooNode: {} > {} => Above({})", input, self.threshold, out);
            Ok(Transition::Next(
                FooAction::Above(out)
            ))
        } else {
            let msg = format!("{} <= {}", input, self.threshold);
            println!("FooNode: {} <= {} => Below(\"{}\")", input, self.threshold, msg);
            Ok(Transition::Next(
                FooAction::Below(msg),
            ))
        }
    }
}

/// Node that handles values above threshold
#[derive(Clone, Debug)]
pub struct BigNode;

#[async_trait]
impl Node for BigNode {
    type Input = u64;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &C,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        println!("BigNode: handling value {}", input);
        Ok(Transition::Finish)
    }
}

/// Node that handles values below threshold
#[derive(Clone, Debug)]
pub struct SmallNode;

#[async_trait]
impl Node for SmallNode {
    type Input = String;
    type Output = ();

    async fn process<C>(
        &self,
        _ctx: &C,
        input: String,
    ) -> Result<Transition<Self::Output>, FloxideError>
    where
        Self: Sized + Node,
        C: Send + Sync + 'static,
    {
        println!("SmallNode: handling message \"{}\"", input);
        Ok(Transition::Finish)
    }
}

/// Manual workflow implementation demonstrating threshold branching
#[derive(Clone, Debug)]
pub struct ThresholdWorkflow {
    foo: FooNode,
    big: BigNode,
    small: SmallNode,
}

#[async_trait]
impl Workflow for ThresholdWorkflow {
    type Input = u64;
    /// Output type of the workflow (unit for side-effect only example)
    type Output = ();

    async fn run<D>(
        &self,
        ctx: &D,
        input: u64,
    ) -> Result<Self::Output, FloxideError>
    where D: Clone + Send + Sync + 'static {
        // Worklist of items tagged by origin
        enum WorkItem {
            Foo(u64),
            Big(u64),
            Small(String),
        }

        let mut work = VecDeque::new();
        work.push_back(WorkItem::Foo(input));

        while let Some(item) = work.pop_front() {
            match item {
                WorkItem::Foo(x) => {
                    match self.foo.process(ctx, x).await? {
                        Transition::Next(FooAction::Above(v)) => {
                            work.push_back(WorkItem::Big(v));
                        }
                        Transition::Next(FooAction::Below(s)) => {
                            work.push_back(WorkItem::Small(s));
                        }
                        Transition::Finish => break,
                        Transition::Abort(e) => return Err(e),
                    }
                }
                WorkItem::Big(v) => {
                    match self.big.process(ctx, v).await? {
                        Transition::Next(_) | Transition::Finish => break,
                        Transition::Abort(e) => return Err(e),
                    }
                }
                WorkItem::Small(s) => {
                    match self.small.process(ctx, s).await? {
                        Transition::Next(_) | Transition::Finish => break,
                        Transition::Abort(e) => return Err(e),
                    }
                }
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wf = ThresholdWorkflow {
        foo: FooNode { threshold: 10 },
        big: BigNode,
        small: SmallNode,
    };
    let ctx = WorkflowCtx::new(());
    println!("Running with input 5:");
    wf.run(&ctx, 5).await?;
    println!("Running with input 42:");
    wf.run(&ctx, 42).await?;
    Ok(())
}
