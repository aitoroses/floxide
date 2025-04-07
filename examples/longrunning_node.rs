//! Long-Running Node Pattern: Suspendable and Resumable Processing
//!
//! This example demonstrates how to implement and use the Long-Running Node pattern
//! in the Flow Framework. The Long-Running Node pattern allows for processes that
//! can be suspended and resumed, making it ideal for:
//!
//! Key concepts demonstrated:
//! 1. Stateful processing that can be paused and resumed
//! 2. Checkpointing of progress during long-running operations
//! 3. State persistence and restoration
//! 4. Progress tracking and reporting
//! 5. Graceful handling of interruptions
//!
//! The example implements two scenarios:
//! - Basic long-running process: A simple process with checkpoints
//! - Multi-step process: A more complex process with multiple stages
//!
//! Long-running nodes are particularly useful for:
//! - Batch processing of large datasets
//! - Operations that may need to be paused due to resource constraints
//! - Processes that should be resilient to system restarts
//! - Workflows that need to track detailed progress
//!
//! This example is designed in accordance with:
//! - ADR-0018: Long-Running Node Pattern
//! - ADR-0022: State Persistence Strategy

use chrono::{DateTime, Utc};
use floxide_core::{DefaultAction, FloxideError};
use floxide_longrunning::{
    InMemoryStateStore, LongRunningOutcome, SimpleLongRunningNode, StateStore,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, Level};

/// Context for tracking process execution
///
/// This struct maintains the state of a process being executed,
/// including the total number of steps, completed steps, and results.
#[derive(Debug, Clone)]
struct ProcessContext {
    /// Total number of steps in the process
    _total_steps: u32,
    /// Number of steps completed so far
    completed_steps: u32,
    /// Unique identifier for this process
    _process_id: String,
    /// Results collected during process execution
    _results: Vec<String>,
}

impl ProcessContext {
    /// Creates a new process context with the specified number of steps
    fn new(total_steps: u32, process_id: impl Into<String>) -> Self {
        Self {
            _total_steps: total_steps,
            completed_steps: 0,
            _process_id: process_id.into(),
            _results: Vec::new(),
        }
    }

    /// Increments the completed step counter
    fn increment_step(&mut self) {
        self.completed_steps += 1;
    }

    /// Adds a result to the results collection
    fn _add_result(&mut self, result: impl Into<String>) {
        self._results.push(result.into());
    }

    /// Checks if all steps have been completed
    fn _is_complete(&self) -> bool {
        self.completed_steps >= self._total_steps
    }
}

/// Serializable state for a long-running process
///
/// This struct represents the persistent state of a long-running process.
/// It can be serialized/deserialized to allow the process to be suspended and resumed.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessState {
    /// Current step being executed
    current_step: u32,
    /// Total number of steps in the process
    total_steps: u32,
    /// Data collected at each checkpoint
    checkpoint_data: HashMap<String, String>,
    /// Timestamp of the last state update
    last_updated: DateTime<Utc>,
}

impl ProcessState {
    /// Creates a new process state with the specified number of steps
    fn new(total_steps: u32) -> Self {
        Self {
            current_step: 0,
            total_steps,
            checkpoint_data: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    /// Adds data to the checkpoint collection
    fn add_checkpoint_data(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.checkpoint_data.insert(key.into(), value.into());
    }

    /// Increments the current step and updates the timestamp
    fn increment_step(&mut self) {
        self.current_step += 1;
        self.last_updated = Utc::now();
    }

    /// Checks if all steps have been completed
    fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }
}

/// Context for file processing operations
///
/// This struct maintains information about files being processed
/// and the progress of each file processing operation.
#[derive(Debug, Clone)]
struct FileProcessingContext {
    /// List of files to be processed
    files: Vec<String>,
    /// Information about each file processing operation
    processes: HashMap<String, FileProcessInfo>,
}

/// Information about a file processing operation
///
/// This struct contains details about a file being processed,
/// including its path, size, and processing progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileProcessInfo {
    /// Path to the file being processed
    file_path: String,
    /// Total size of the file in bytes
    file_size: usize,
    /// Number of bytes processed so far
    processed_bytes: usize,
    /// Timestamp when processing started
    started_at: DateTime<Utc>,
    /// Timestamp of the last checkpoint
    last_checkpoint: DateTime<Utc>,
}

impl FileProcessingContext {
    /// Creates a new file processing context with the specified files
    fn new(files: Vec<String>) -> Self {
        Self {
            files,
            processes: HashMap::new(),
        }
    }
}

/// Main function that demonstrates the long-running node pattern
#[tokio::main]
async fn main() -> Result<(), FloxideError> {
    // Initialize the tracing subscriber for logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Run the basic example
    info!("Running basic long-running example");
    run_basic_longrunning_example().await?;

    // Run the multi-step example
    info!("\nRunning multi-step process example");
    run_multi_step_process_example().await?;

    Ok(())
}

/// Demonstrates a basic long-running process with checkpoints
///
/// This example shows:
/// 1. Creating a simple long-running node
/// 2. Executing the node with checkpoints
/// 3. Suspending and resuming execution
/// 4. Handling completion
async fn run_basic_longrunning_example() -> Result<(), FloxideError> {
    // Create a context for the process
    let mut context = ProcessContext::new(5, "basic-process");

    // Create an in-memory state store
    let state_store = InMemoryStateStore::new();

    // Create a long-running node that processes in steps
    let node = SimpleLongRunningNode::new(
        "basic-process",
        |ctx: &mut ProcessContext, state: &mut ProcessState| async move {
            // Get the current step
            let step = state.current_step;

            info!("Executing step {} of {}", step + 1, state.total_steps);

            // Simulate some work
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // Update the context and state
            ctx.increment_step();
            state.increment_step();

            // Add checkpoint data
            state.add_checkpoint_data(
                format!("step_{}", step),
                format!("Completed step {} at {}", step, Utc::now()),
            );

            // Check if we're done
            if state.is_complete() {
                info!("Process complete!");
                Ok(LongRunningOutcome::Complete(DefaultAction::Next))
            } else {
                // Simulate suspending after step 2
                if step == 2 {
                    info!("Suspending after step 3");
                    Ok(LongRunningOutcome::Suspend)
                } else {
                    Ok(LongRunningOutcome::Continue)
                }
            }
        },
    );

    // Execute the node until it suspends or completes
    info!("Starting execution");
    let result = state_store.execute_async(&node, &mut context).await?;

    match result {
        LongRunningOutcome::Suspend => {
            info!("Process suspended after step {}", context.completed_steps);

            // Resume the process
            info!("Resuming execution");
            let resume_result = state_store.execute_async(&node, &mut context).await?;

            match resume_result {
                LongRunningOutcome::Complete(action) => {
                    info!("Process completed after resuming with action: {:?}", action);
                }
                _ => {
                    info!("Process still not complete after resuming");
                }
            }
        }
        LongRunningOutcome::Complete(action) => {
            info!("Process completed in one go with action: {:?}", action);
        }
        LongRunningOutcome::Continue => {
            info!("Process continued but did not complete (unexpected)");
        }
    }

    Ok(())
}

/// Demonstrates a multi-step process with complex state management
///
/// This example shows:
/// 1. Creating a long-running node with multiple processing stages
/// 2. Managing complex state across suspensions
/// 3. Reporting detailed progress information
/// 4. Handling completion with different outcomes
async fn run_multi_step_process_example() -> Result<(), FloxideError> {
    // Create a context with files to process
    let files = vec![
        "file1.txt".to_string(),
        "file2.txt".to_string(),
        "file3.txt".to_string(),
    ];
    let mut context = FileProcessingContext::new(files);

    // Create an in-memory state store
    let state_store = InMemoryStateStore::new();

    // Create a long-running node that processes files in chunks
    let node = SimpleLongRunningNode::new(
        "file-processor",
        |ctx: &mut FileProcessingContext, state: &mut ProcessState| async move {
            // Initialize state if this is the first run
            if state.current_step == 0 {
                info!("Initializing file processing");
                state.total_steps = ctx.files.len() as u32;

                // Set up initial file info
                for (i, file) in ctx.files.iter().enumerate() {
                    let file_info = FileProcessInfo {
                        file_path: file.clone(),
                        file_size: 1000 * (i + 1), // Simulate different file sizes
                        processed_bytes: 0,
                        started_at: Utc::now(),
                        last_checkpoint: Utc::now(),
                    };

                    ctx.processes.insert(file.clone(), file_info);

                    // Add initial checkpoint data
                    state.add_checkpoint_data(
                        format!("file_{}_init", i),
                        format!("Initialized file {} with size {}", file, 1000 * (i + 1)),
                    );
                }
            }

            // Get the current file being processed
            let current_file_index = state.current_step as usize;
            if current_file_index >= ctx.files.len() {
                info!("All files processed!");
                return Ok(LongRunningOutcome::Complete(DefaultAction::Next));
            }

            let current_file = &ctx.files[current_file_index];
            info!("Processing file: {}", current_file);

            // Get the file info
            let mut file_info = ctx.processes.get(current_file).unwrap().clone();

            // Simulate processing the file in chunks
            let chunk_size = 250;
            let mut bytes_processed = file_info.processed_bytes;

            while bytes_processed < file_info.file_size {
                // Process a chunk
                let new_bytes = std::cmp::min(chunk_size, file_info.file_size - bytes_processed);
                bytes_processed += new_bytes;

                // Update file info
                file_info.processed_bytes = bytes_processed;
                file_info.last_checkpoint = Utc::now();

                // Update the context
                ctx.processes
                    .insert(current_file.clone(), file_info.clone());

                // Add checkpoint data
                state.add_checkpoint_data(
                    format!("file_{}_progress", current_file_index),
                    format!(
                        "Processed {}/{} bytes of {}",
                        bytes_processed, file_info.file_size, current_file
                    ),
                );

                info!(
                    "Progress: {}/{} bytes ({}%)",
                    bytes_processed,
                    file_info.file_size,
                    (bytes_processed * 100) / file_info.file_size
                );

                // Simulate work
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                // Suspend after processing half of file 1
                if current_file_index == 0 && bytes_processed == chunk_size * 2 {
                    info!("Suspending in the middle of file 1");
                    return Ok(LongRunningOutcome::Suspend);
                }
            }

            // File is complete
            info!("Completed processing file: {}", current_file);

            // Move to the next file
            state.increment_step();

            // Continue processing
            Ok(LongRunningOutcome::Continue)
        },
    );

    // Execute the node until it suspends or completes
    info!("Starting file processing");
    let result = state_store.execute_async(&node, &mut context).await?;

    match result {
        LongRunningOutcome::Suspend => {
            info!("Processing suspended");

            // Print current progress
            for (file, info) in &context.processes {
                info!(
                    "File {}: {}/{} bytes ({}%)",
                    file,
                    info.processed_bytes,
                    info.file_size,
                    (info.processed_bytes * 100) / info.file_size
                );
            }

            // Resume processing
            info!("Resuming file processing");
            let resume_result = state_store.execute_async(&node, &mut context).await?;

            match resume_result {
                LongRunningOutcome::Complete(action) => {
                    info!(
                        "File processing completed after resuming with action: {:?}",
                        action
                    );
                }
                _ => {
                    info!("File processing still not complete after resuming");
                }
            }
        }
        LongRunningOutcome::Complete(action) => {
            info!(
                "File processing completed in one go with action: {:?}",
                action
            );
        }
        LongRunningOutcome::Continue => {
            info!("File processing continued but did not complete (unexpected)");
        }
    }

    // Print final progress
    info!("Final processing status:");
    for (file, info) in &context.processes {
        info!(
            "File {}: {}/{} bytes ({}%)",
            file,
            info.processed_bytes,
            info.file_size,
            (info.processed_bytes * 100) / info.file_size
        );
    }

    Ok(())
}

// Extension trait to make examples more readable
trait StateStoreExt {
    /// Executes a long-running node asynchronously until it suspends or completes
    ///
    /// This helper method simplifies the execution of long-running nodes by handling
    /// the initialization, execution, and result processing in a single method call.
    async fn execute_async<N, C, O>(
        &self,
        node: &N,
        ctx: &mut C,
    ) -> Result<LongRunningOutcome<O, N::State>, FloxideError>
    where
        N: SimpleLongRunningNode<C, DefaultAction, Output = O>,
        O: Clone + Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
        N::State: Clone + Send + Sync + 'static;
}

impl<S: StateStore> StateStoreExt for S {
    async fn execute_async<N, C, O>(
        &self,
        node: &N,
        ctx: &mut C,
    ) -> Result<LongRunningOutcome<O, N::State>, FloxideError>
    where
        N: SimpleLongRunningNode<C, DefaultAction, Output = O>,
        O: Clone + Send + Sync + 'static,
        C: Clone + Send + Sync + 'static,
        N::State: Clone + Send + Sync + 'static,
    {
        // Get or initialize the node state
        let mut state = match self.get_state::<N::State>(node.id()).await? {
            Some(state) => {
                info!(
                    "Resuming from existing state at step {}",
                    state.current_step
                );
                state
            }
            None => {
                info!("Initializing new state");
                let state = N::State::new(0);
                self.save_state(node.id(), &state).await?;
                state
            }
        };

        // Execute the node
        let mut outcome = node.execute(ctx, &mut state).await?;

        // Continue execution until suspended or completed
        loop {
            match &outcome {
                LongRunningOutcome::Continue => {
                    // Save the state
                    self.save_state(node.id(), &state).await?;

                    // Continue execution
                    outcome = node.execute(ctx, &mut state).await?;
                }
                LongRunningOutcome::Suspend => {
                    // Save the state and return
                    self.save_state(node.id(), &state).await?;
                    break;
                }
                LongRunningOutcome::Complete(_) => {
                    // Clear the state and return
                    self.clear_state(node.id()).await?;
                    break;
                }
            }
        }

        Ok(outcome)
    }
}
