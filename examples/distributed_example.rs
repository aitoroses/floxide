// examples/distributed_example.rs
// Demonstrates running a floxide workflow in-memory with distributed execution.
use async_trait::async_trait;
use floxide_core::*;
use floxide_macros::workflow;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};
use tokio::time::Duration;

// In-memory work-queue implementation
#[derive(Clone)]
struct InMemQueue<W>(Arc<Mutex<HashMap<String, VecDeque<W>>>>);
impl<W: Clone + Send + 'static> InMemQueue<W> {
    fn new() -> Self {
        InMemQueue(Arc::new(Mutex::new(HashMap::new())))
    }
}
impl<W: Clone + Send + 'static> floxide_core::distributed::WorkQueue<W> for InMemQueue<W> {
    fn enqueue(&self, workflow_id: &str, work: W) -> Result<(), String> {
        let mut map = self.0.lock().unwrap();
        map.entry(workflow_id.to_string())
            .or_default()
            .push_back(work);
        Ok(())
    }
    fn dequeue(&self, workflow_id: &str) -> Result<Option<W>, String> {
        let mut map = self.0.lock().unwrap();
        Ok(map.get_mut(workflow_id).and_then(|q| q.pop_front()))
    }
}

// In-memory checkpoint store
#[derive(Clone)]
struct InMemStore<C: Clone + Send + 'static, W: Clone + Send + 'static>(
    Arc<Mutex<HashMap<String, Checkpoint<C, W>>>>,
);
impl<C: Clone + Send + 'static, W: Clone + Send + 'static> InMemStore<C, W> {
    fn new() -> Self {
        InMemStore(Arc::new(Mutex::new(HashMap::new())))
    }
}
impl<C: Clone + Send + 'static, W: Clone + Send + 'static> CheckpointStore<C, W>
    for InMemStore<C, W>
{
    fn save(
        &self,
        workflow_id: &str,
        checkpoint: &Checkpoint<C, W>,
    ) -> Result<(), CheckpointError> {
        let mut map = self.0.lock().unwrap();
        map.insert(workflow_id.to_string(), checkpoint.clone());
        Ok(())
    }
    fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<C, W>>, CheckpointError> {
        let map = self.0.lock().unwrap();
        Ok(map.get(workflow_id).cloned())
    }
}

// Simple workflow from simple_context_example
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyCtx {
    pub value: u64,
}
#[derive(Clone, Debug)]
pub enum FooAction {
    Above(u64),
    Below(String),
}
#[derive(Clone, Debug)]
pub struct FooNode {
    threshold: u64,
}
#[async_trait]
impl Node<MyCtx> for FooNode {
    type Input = u64;
    type Output = FooAction;
    async fn process(
        &self,
        _ctx: &MyCtx,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        if input > self.threshold {
            Ok(Transition::Next(FooAction::Above(input * 2)))
        } else {
            Ok(Transition::Next(FooAction::Below(format!(
                "{} <= {}",
                input, self.threshold
            ))))
        }
    }
}
#[derive(Clone, Debug)]
pub struct BigNode;
#[async_trait]
impl Node<MyCtx> for BigNode {
    type Input = u64;
    type Output = ();
    async fn process(
        &self,
        _ctx: &MyCtx,
        input: u64,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("BigNode got {}", input);
        Ok(Transition::Next(()))
    }
}
#[derive(Clone, Debug)]
pub struct SmallNode;
#[async_trait]
impl Node<MyCtx> for SmallNode {
    type Input = String;
    type Output = ();
    async fn process(
        &self,
        _ctx: &MyCtx,
        input: String,
    ) -> Result<Transition<Self::Output>, FloxideError> {
        println!("SmallNode got {}", input);
        Ok(Transition::Next(()))
    }
}

workflow! {
    pub struct ThresholdWorkflow {
        foo: FooNode,
        big: BigNode,
        small: SmallNode,
    }
    start = foo;
    context = MyCtx;
    edges {
        foo => {
            FooAction::Above(v) => [ big ];
            FooAction::Below(s) => [ small ];
        };
        big => {};
        small => {};
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    // Create in-memory runtime
    let store = InMemStore::<MyCtx, ThresholdWorkflowWorkItem>::new();
    let queue = InMemQueue::<ThresholdWorkflowWorkItem>::new();
    // Construct workflow & context
    let wf = ThresholdWorkflow {
        foo: FooNode { threshold: 10 },
        big: BigNode,
        small: SmallNode,
    };
    let ctx = WorkflowCtx::new(MyCtx { value: 0 });
    let run_id = "run1";
    // Start distributed run (seed checkpoint+queue)
    wf.start_distributed(&ctx, 42u64, &store, &queue, run_id)
        .await?;
    // Spawn workers to process steps
    for i in 0..2 {
        let wf = wf.clone();
        let store = store.clone();
        let queue = queue.clone();
        let id = run_id.to_string();
        tokio::spawn(async move {
            loop {
                // each worker passes its own ID
                let step = wf.step_distributed(&store, &queue, &id, i).await
                    .expect("step_distributed failed");
                if let Some(res) = step {
                    println!("Worker {} terminal output: {:?}", i, res);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
    }
    // Wait for completion by polling the checkpoint (simple example)
    loop {
        if let Some(cp) = store.load(run_id)? {
            if cp.queue.is_empty() {
                println!("Final context: {:?}", cp.context);
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}
