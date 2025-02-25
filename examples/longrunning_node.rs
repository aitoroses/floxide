//! Example demonstrating the LongRunningNode functionality for processes that
//! can be suspended and resumed.

use chrono::{DateTime, Utc};
use flowrs_core::{DefaultAction, FlowrsError};
use flowrs_longrunning::{
    InMemoryStateStore, LongRunningOutcome, SimpleLongRunningNode, StateStore,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, Level};

#[derive(Debug, Clone)]
struct ProcessContext {
    _total_steps: u32,
    completed_steps: u32,
    _process_id: String,
    _results: Vec<String>,
}

impl ProcessContext {
    fn new(total_steps: u32, process_id: impl Into<String>) -> Self {
        Self {
            _total_steps: total_steps,
            completed_steps: 0,
            _process_id: process_id.into(),
            _results: Vec::new(),
        }
    }

    fn increment_step(&mut self) {
        self.completed_steps += 1;
    }

    fn _add_result(&mut self, result: impl Into<String>) {
        self._results.push(result.into());
    }

    fn _is_complete(&self) -> bool {
        self.completed_steps >= self._total_steps
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessState {
    current_step: u32,
    total_steps: u32,
    checkpoint_data: HashMap<String, String>,
    last_updated: DateTime<Utc>,
}

impl ProcessState {
    fn new(total_steps: u32) -> Self {
        Self {
            current_step: 0,
            total_steps,
            checkpoint_data: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    fn add_checkpoint_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.checkpoint_data.insert(key.into(), value.into());
    }

    fn increment_step(&mut self) {
        self.current_step += 1;
        self.last_updated = Utc::now();
    }

    fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }
}

#[derive(Debug, Clone)]
struct FileProcessingContext {
    files: Vec<String>,
    processes: HashMap<String, FileProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileProcessInfo {
    file_path: String,
    file_size: usize,
    processed_bytes: usize,
    started_at: DateTime<Utc>,
    last_checkpoint: DateTime<Utc>,
}

impl FileProcessingContext {
    fn new(files: Vec<String>) -> Self {
        Self {
            files,
            processes: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), FlowrsError> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Run examples
    info!("Running basic long-running example...");
    run_basic_longrunning_example().await?;

    info!("Running multi-step process example...");
    run_multi_step_process_example().await?;

    Ok(())
}

async fn run_basic_longrunning_example() -> Result<(), FlowrsError> {
    // Create an in-memory state store
    let state_store = InMemoryStateStore::new();

    // Create a simple process node that needs 3 steps to complete
    let process_node =
        SimpleLongRunningNode::<ProcessContext, DefaultAction, ProcessState, &str, _>::with_id(
            "step_processor",
            |state: Option<ProcessState>, ctx: &mut ProcessContext| {
                // Get current state or create a new one
                let mut current_state = match state {
                    Some(state) => state.clone(),
                    None => ProcessState::new(3),
                };

                // Process the current step
                info!(
                    "Processing step {}/{}, ctx.completed_steps: {}",
                    current_state.current_step + 1,
                    current_state.total_steps,
                    ctx.completed_steps
                );

                // Simulate some work
                ctx.increment_step();
                current_state.add_checkpoint_data(
                    format!("step_{}", current_state.current_step),
                    format!("Processed at {}", Utc::now()),
                );
                current_state.increment_step();

                // Check if processing is complete
                if current_state.is_complete() {
                    info!(
                        "Process complete after {} steps",
                        current_state.current_step
                    );
                    Ok(LongRunningOutcome::Complete(
                        "Process completed successfully",
                    ))
                } else {
                    info!(
                        "Process suspended at step {}/{}",
                        current_state.current_step, current_state.total_steps
                    );
                    Ok(LongRunningOutcome::Suspend(current_state))
                }
            },
        );

    // Create a new process context
    let mut ctx = ProcessContext::new(3, "basic-process");

    // First execution - should suspend
    info!("First execution");
    let outcome = state_store
        .execute_async(&process_node, &mut ctx)
        .await
        .unwrap();
    info!("First execution outcome: {:?}", outcome);

    // Second execution - should suspend
    info!("Second execution");
    let outcome = state_store
        .execute_async(&process_node, &mut ctx)
        .await
        .unwrap();
    info!("Second execution outcome: {:?}", outcome);

    // Third execution - should complete
    info!("Third execution");
    let outcome = state_store
        .execute_async(&process_node, &mut ctx)
        .await
        .unwrap();
    info!("Third execution outcome: {:?}", outcome);

    Ok(())
}

async fn run_multi_step_process_example() -> Result<(), FlowrsError> {
    // Create an in-memory state store
    let state_store = InMemoryStateStore::new();

    // Create a list of files to process
    let files = vec![
        "file1.txt".to_string(),
        "file2.txt".to_string(),
        "file3.txt".to_string(),
    ];

    // Create a process context
    let mut ctx = FileProcessingContext::new(files);

    // Create a node that processes files in chunks
    let batch_processor = SimpleLongRunningNode::<
        FileProcessingContext,
        DefaultAction,
        HashMap<String, FileProcessInfo>,
        HashMap<String, FileProcessInfo>,
        _,
    >::new(
        move |state: Option<HashMap<String, FileProcessInfo>>, ctx: &mut FileProcessingContext| {
            // Get or initialize state
            let mut state = match state {
                Some(state) => state.clone(),
                None => HashMap::new(),
            };

            // Process or initialize the next file
            if let Some(file) = ctx.files.first() {
                let file_path = file.clone();

                // Get or create processing info for this file
                let process_info = state.entry(file_path.clone()).or_insert_with(|| {
                    info!("Starting to process file: {}", file_path);
                    FileProcessInfo {
                        file_path: file_path.clone(),
                        file_size: 1000, // Simulate file size of 1000 bytes
                        processed_bytes: 0,
                        started_at: Utc::now(),
                        last_checkpoint: Utc::now(),
                    }
                });

                // Simulate processing a chunk of the file
                let chunk_size = 400; // Process 400 bytes at a time
                process_info.processed_bytes = std::cmp::min(
                    process_info.processed_bytes + chunk_size,
                    process_info.file_size,
                );
                process_info.last_checkpoint = Utc::now();

                info!(
                    "Processed {}/{} bytes of file {}",
                    process_info.processed_bytes, process_info.file_size, file_path
                );

                // Check if file processing is complete
                if process_info.processed_bytes >= process_info.file_size {
                    info!("Completed processing file: {}", file_path);
                    // Remove the completed file and its process info
                    ctx.files.remove(0);
                    ctx.processes
                        .insert(file_path.clone(), process_info.clone());
                    state.remove(&file_path);
                }

                // Return appropriate outcome
                if ctx.files.is_empty() {
                    info!("All files processed!");
                    Ok(LongRunningOutcome::Complete(ctx.processes.clone()))
                } else {
                    info!("Suspended processing. {} files remaining.", ctx.files.len());
                    Ok(LongRunningOutcome::Suspend(state))
                }
            } else {
                // No files to process
                info!("No files to process!");
                Ok(LongRunningOutcome::Complete(ctx.processes.clone()))
            }
        },
    );

    // Process all files in multiple steps
    let mut execution_count = 1;
    loop {
        info!("Execution #{}", execution_count);
        let outcome = state_store
            .execute_async(&batch_processor, &mut ctx)
            .await
            .unwrap();

        match outcome {
            LongRunningOutcome::Complete(_) => {
                info!("All files processed successfully!");
                break;
            }
            LongRunningOutcome::Suspend(_) => {
                info!("Processing suspended. Will resume...");
                execution_count += 1;
            }
        }
    }

    Ok(())
}

// Extension trait to make examples more readable
trait StateStoreExt: StateStore {
    async fn execute_async<N, C, A, O>(
        &self,
        node: &N,
        ctx: &mut C,
    ) -> Result<LongRunningOutcome<O, N::State>, FlowrsError>
    where
        N: flowrs_longrunning::LongRunningNode<C, A, Output = O>,
        C: Send + Sync + 'static,
        A: flowrs_core::ActionType + Send + Sync + 'static + std::fmt::Debug;
}

impl<S: StateStore> StateStoreExt for S {
    async fn execute_async<N, C, A, O>(
        &self,
        node: &N,
        ctx: &mut C,
    ) -> Result<LongRunningOutcome<O, N::State>, FlowrsError>
    where
        N: flowrs_longrunning::LongRunningNode<C, A, Output = O>,
        C: Send + Sync + 'static,
        A: flowrs_core::ActionType + Send + Sync + 'static + std::fmt::Debug,
    {
        // Get node's state if it exists
        let node_id = node.id();
        let state = self.get_state::<N::State>(node_id.clone()).await?;

        // Process the node
        let outcome = node.process(state, ctx).await?;

        // Save or remove state based on outcome
        match &outcome {
            LongRunningOutcome::Complete(_) => {
                self.remove_state(node_id).await?;
            }
            LongRunningOutcome::Suspend(state) => {
                self.save_state(node_id, state).await?;
            }
        }

        Ok(outcome)
    }
}
