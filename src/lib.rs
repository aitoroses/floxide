//! # Flowrs - A directed graph workflow system in Rust
//!
//! This crate re-exports the core components of the Flowrs workflow system.
//! It serves as the entry point for using Flowrs in your applications.

// Re-export core components
pub use flowrs_core::*;

// Re-export transform functionality
pub use flowrs_transform::*;

// Re-export event functionality
pub use flowrs_event::*;

// Re-export timer functionality
pub use flowrs_timer::*;

// Re-export longrunning functionality
pub use flowrs_longrunning::*;

// Re-export reactive functionality
pub use flowrs_reactive::*;

// Init function for the framework
pub fn init() {
    // Initialize tracing for better logs
    tracing_subscriber::fmt::init();
}
