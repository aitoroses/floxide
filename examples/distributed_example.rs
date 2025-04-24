// examples/distributed_example.rs
// Demonstrates running a floxide workflow in-memory with distributed execution.
use async_trait::async_trait;
use floxide_core::*;
use floxide_macros::workflow;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque}, fmt::{self, Debug}, sync::{Arc, Mutex}
};
use tokio::time::Duration;
use tracing::Instrument;


#[derive(Clone, Debug)]
struct Ctx {
    local_counter: ArcMutex<i32>,
    logs: ArcMutex<Vec<String>>,
}

// In-memory work-queue implementation
#[derive(Clone)]
struct InMemQueue<W>(Arc<tokio::sync::Mutex<HashMap<String, VecDeque<W>>>>);
impl<W: Clone + Send + 'static> InMemQueue<W> {
    fn new() -> Self {
        InMemQueue(Arc::new(tokio::sync::Mutex::new(HashMap::new())))
    }
}

#[async_trait]
impl<W: Clone + Send + 'static> floxide_core::distributed::WorkQueue<W> for InMemQueue<W> {
    async fn enqueue(&self, workflow_id: &str, work: W) -> Result<(), String> {
        let mut map = self.0.lock().await;
        map.entry(workflow_id.to_string())
            .or_default()
            .push_back(work);
        Ok(())
    }
    async fn dequeue(&self) -> Result<Option<(String, W)>, String> {
        let mut map = self.0.lock().await;
        // find any non-empty queue entry
        for (run_id, q) in map.iter_mut() {
            if let Some(item) = q.pop_front() {
                return Ok(Some((run_id.clone(), item)));
            }
        }
        Ok(None)
    }
}

// In-memory checkpoint store
#[derive(Clone)]
struct InMemStore<W: Clone + Send + 'static>(
    Arc<tokio::sync::Mutex<HashMap<String, Checkpoint<Ctx, W>>>>,
);

impl<W: Clone + Send + 'static> InMemStore<W> {
    fn new() -> Self {
        InMemStore(Arc::new(tokio::sync::Mutex::new(HashMap::new())))
    }
}

#[async_trait]
impl<W: Clone + Send + Sync> CheckpointStore<Ctx, W> for InMemStore<W> {
    async fn save(
        &self,
        workflow_id: &str,
        checkpoint: &Checkpoint<Ctx, W>,
    ) -> Result<(), CheckpointError> {
        let mut map = self.0.lock().await;
        map.insert(workflow_id.to_string(), checkpoint.clone());
        Ok(())
    }
    async fn load(&self, workflow_id: &str) -> Result<Option<Checkpoint<Ctx, W>>, CheckpointError> {
        let map = self.0.lock().await;
        Ok(map.get(workflow_id).cloned())
    }
}

#[derive(Clone)]
pub struct ArcMutex<T>(Arc<Mutex<T>>);

impl<T> ArcMutex<T> {
    pub fn new(value: T) -> Self {
        ArcMutex(Arc::new(Mutex::new(value)))
    }
}

impl<T: Serialize + Clone> Serialize for ArcMutex<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Arc<Mutex<T>>", 1)?;
        let value = self.0.lock().unwrap();
        state.serialize_field("value", &value.clone())?;
        state.end()
    }
}

impl<'de, T: Deserialize<'de> + Clone> Deserialize<'de> for ArcMutex<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = T::deserialize(deserializer)?;
        Ok(ArcMutex(Arc::new(Mutex::new(value))))
    }
}

impl<T: Debug> Debug for ArcMutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0.lock().unwrap())
    }
}

// === Parallel workflow example illustrating worker collaboration ===
// Define simple nodes: split into two parallel branches, then terminal nodes and an initial node

#[derive(Clone, Debug)]
pub struct InitialNode;
#[async_trait]
impl Node<Ctx> for InitialNode {
    type Input = ();
    type Output = ();
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("InitialNode: starting workflow");
        let mut counter = ctx.local_counter.0.lock().unwrap();
        *counter += 1;
        let mut logs = ctx.logs.0.lock().unwrap();
        logs.push(format!("InitialNode: starting workflow"));
        Ok(Transition::Next(()))
    }
}

#[derive(Clone, Debug)]
pub struct SplitNode;
#[async_trait]
impl Node<Ctx> for SplitNode {
    type Input = ();
    type Output = ();
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("SplitNode: spawning two branches");
        let mut counter = ctx.local_counter.0.lock().unwrap();
        *counter += 1;
        let mut logs = ctx.logs.0.lock().unwrap();
        logs.push(format!("SplitNode: spawning two branches"));
        Ok(Transition::Next(()))
    }
}
#[derive(Clone, Debug)]
pub struct BranchA;
#[async_trait]
impl Node<Ctx> for BranchA {
    type Input = ();
    type Output = ();
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("BranchA executed");
        let mut counter = ctx.local_counter.0.lock().unwrap();
        *counter += 10;
        let mut logs = ctx.logs.0.lock().unwrap();
        logs.push(format!("BranchA executed"));
        Ok(Transition::Next(()))
    }
}
#[derive(Clone, Debug)]
pub struct BranchB;
#[async_trait]
impl Node<Ctx> for BranchB {
    type Input = ();
    type Output = ();
    async fn process(
        &self,
        ctx: &Ctx,
        _input: (),
    ) -> Result<Transition<Self::Output>, FloxideError> {
        tracing::info!("BranchB executed");
        let mut counter = ctx.local_counter.0.lock().unwrap();
        *counter += 15;
        let mut logs = ctx.logs.0.lock().unwrap();
        logs.push(format!("BranchB executed"));
        Ok(Transition::Next(()))
    }
}

workflow! {
    pub struct ParallelWorkflow {
        initial: InitialNode,
        split: SplitNode,
        a: BranchA,
        b: BranchB,
    }
    start = initial;
    context = Ctx;
    edges {
        initial => { [ split ] };
        split => {[ a, b ]};
        a => {};
        b => {};
    }
}

async fn run_distributed_example() -> Result<Ctx, Box<dyn std::error::Error>> {
    // Create in-memory runtime
    let store = InMemStore::<ParallelWorkflowWorkItem>::new();
    let queue = InMemQueue::<ParallelWorkflowWorkItem>::new();

    // Build workflow and context
    let wf = ParallelWorkflow {
        initial: InitialNode,
        split: SplitNode,
        a: BranchA,
        b: BranchB,
    };
    let ctx = WorkflowCtx::new(Ctx {
        local_counter: ArcMutex::new(0),
        logs: ArcMutex::new(Vec::new()),
    });
    let run_id = "run1";
    // Seed the single run, enqueuing the split node
    wf.start_distributed(&ctx, (), &store, &queue, run_id)
        .await?;
    // Spawn two workers to process distributed steps and collect their JoinHandles
    let mut handles = Vec::new();
    for i in 0..2 {
        let wf = wf.clone();
        let store = store.clone();
        let queue = queue.clone();
        let worker_span = tracing::span!(tracing::Level::DEBUG, "worker_task", worker = i);
        let handle = tokio::spawn(
            async move {
                // Process until this worker sees its terminal branch event
                loop {
                    if let Some((run, _res)) = wf
                        .step_distributed(&store, &queue, i)
                        .await
                        .expect("step_distributed failed")
                    {
                        println!("Worker {} processed branch of run {}", i, run);
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
            .instrument(worker_span),
        );
        handles.push(handle);
    }

    // Wait for all workers to finish processing
    for handle in handles {
        handle.await.expect("worker task panicked");
    }

    // All work is done; print final context
    if let Some(cp) = store.load(run_id).await? {
        println!("Run {} completed; final context: {:?}", run_id, cp.context);
        Ok(cp.context)
    } else {
        Err(FloxideError::NotStarted.into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    run_distributed_example().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_distributed_example() {
        let ctx = run_distributed_example().await.unwrap();
        assert_eq!(*ctx.local_counter.0.lock().unwrap(), 27);
        assert_eq!(*ctx.logs.0.lock().unwrap(), vec!["InitialNode: starting workflow", "SplitNode: spawning two branches", "BranchA executed", "BranchB executed"]);
    }
}