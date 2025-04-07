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
    InMemoryStateStore, LongRunningNode, LongRunningOutcome, SimpleLongRunningNode,
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
    /// For file processing, track bytes processed per file
    file_bytes_processed: HashMap<String, usize>,
    /// Flag to indicate if initialization has been done
    initialized: bool,
}

impl ProcessState {
    /// Creates a new process state with the specified number of steps
    fn new(total_steps: u32) -> Self {
        Self {
            current_step: 0,
            total_steps,
            checkpoint_data: HashMap::new(),
            last_updated: Utc::now(),
            file_bytes_processed: HashMap::new(),
            initialized: false,
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

    println!("Starting longrunning_node example...");

    // Simple test to verify stdout is working
    for i in 1..=5 {
        println!("Test print {}", i);
    }

    // Run the basic example
    println!("Running basic long-running example");
    info!("Running basic long-running example");
    run_basic_longrunning_example().await?;

    // Run the multi-step example
    println!("\nRunning multi-step process example");
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
    let _state_store = InMemoryStateStore::new();

    // Create a long-running node that processes in steps
    let node = SimpleLongRunningNode::<_, DefaultAction, _, _, _>::new(
        |state_opt: Option<ProcessState>, ctx: &mut ProcessContext| {
            // Initialize or get state
            let mut state = state_opt.unwrap_or_else(|| ProcessState::new(5));

            // Get the current step
            let step = state.current_step;

            println!("Executing step {} of {}", step + 1, state.total_steps);
            info!("Executing step {} of {}", step + 1, state.total_steps);

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
                println!("Process complete!");
                info!("Process complete!");
                Ok(LongRunningOutcome::Complete(DefaultAction::Next))
            } else {
                // Simulate suspending after step 2
                if step == 2 {
                    println!("Suspending after step 3");
                    info!("Suspending after step 3");
                }
                // For all steps, suspend to continue processing
                Ok(LongRunningOutcome::Suspend(state))
            }
        },
    );

    // Execute the node until it completes
    info!("Starting execution");
    let mut state_opt = None;

    loop {
        let outcome = node.process(state_opt, &mut context).await?;

        match outcome {
            LongRunningOutcome::Complete(action) => {
                info!("Process completed with action: {:?}", action);
                break;
            }
            LongRunningOutcome::Suspend(state) => {
                state_opt = Some(state);
                // If we're at step 3, pause and show a message
                if state_opt.as_ref().unwrap().current_step == 3 {
                    info!("Process suspended after step {}", context.completed_steps);
                    info!("Resuming execution");
                }
            }
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

    // Create a long-running node that processes files in chunks
    let node = SimpleLongRunningNode::<_, DefaultAction, _, _, _>::with_id(
        "file-processor",
        |state_opt: Option<ProcessState>, ctx: &mut FileProcessingContext| {
            // Initialize or get state
            let mut state = state_opt.unwrap_or_else(|| {
                let mut s = ProcessState::new(0);
                s.initialized = false;
                s
            });

            // Initialize state if this is the first run
            if !state.initialized {
                info!("Initializing file processing");
                state.total_steps = ctx.files.len() as u32;
                state.initialized = true;

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
                    state.file_bytes_processed.insert(file.clone(), 0);

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

            // Get the bytes processed so far
            let bytes_processed = *state.file_bytes_processed.get(current_file).unwrap_or(&0);

            // Simulate processing the file in chunks
            let chunk_size = 250;
            let new_bytes = std::cmp::min(chunk_size, file_info.file_size - bytes_processed);
            let new_bytes_processed = bytes_processed + new_bytes;

            // Update file info
            file_info.processed_bytes = new_bytes_processed;
            file_info.last_checkpoint = Utc::now();

            // Update the context and state
            ctx.processes.insert(current_file.clone(), file_info.clone());
            state.file_bytes_processed.insert(current_file.clone(), new_bytes_processed);

            // Add checkpoint data
            state.add_checkpoint_data(
                format!("file_{}_progress", current_file_index),
                format!(
                    "Processed {}/{} bytes of {}",
                    new_bytes_processed, file_info.file_size, current_file
                ),
            );

            info!(
                "Progress: {}/{} bytes ({}%)",
                new_bytes_processed,
                file_info.file_size,
                (new_bytes_processed * 100) / file_info.file_size
            );

            // Suspend after processing half of file 1
            if current_file_index == 0 && new_bytes_processed == chunk_size * 2 {
                info!("Suspending in the middle of file 1");
                return Ok(LongRunningOutcome::Suspend(state));
            }

            // Check if file is complete
            if new_bytes_processed >= file_info.file_size {
                // File is complete
                info!("Completed processing file: {}", current_file);

                // Move to the next file
                state.increment_step();
            }

            // Continue processing by suspending with updated state
            Ok(LongRunningOutcome::Suspend(state))
        },
    );

    // Execute the node until it completes
    info!("Starting file processing");
    let mut state_opt = None;
    let mut suspended_at_half = false;

    loop {
        let outcome = node.process(state_opt, &mut context).await?;

        match outcome {
            LongRunningOutcome::Complete(action) => {
                info!("File processing completed with action: {:?}", action);
                break;
            }
            LongRunningOutcome::Suspend(state) => {
                // Check if we're at the suspension point (half of file 1)
                let current_file_index = state.current_step as usize;
                if current_file_index == 0 {
                    let current_file = &context.files[current_file_index];
                    let bytes_processed = *state.file_bytes_processed.get(current_file).unwrap_or(&0);

                    if bytes_processed == 500 && !suspended_at_half {
                        suspended_at_half = true;
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

                        info!("Resuming file processing");
                    }
                }

                state_opt = Some(state);
            }
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
