//! # Floxide - A directed graph workflow system in Rust
//!
//! This crate provides a flexible, type-safe framework for building directed graph workflows.
//! It allows you to create complex processing pipelines with clear transitions between steps.
//!
//! ## Features
//!
//! The crate is organized into feature-gated modules:
//!
//! - `core` (default): Core abstractions and functionality
//! - `transform`: Transform node implementations for data transformation pipelines
//! - `event`: Event-driven workflow functionality
//! - `timer`: Time-based workflow functionality
//! - `longrunning`: Long-running process functionality
//! - `reactive`: Reactive workflow functionality
//! - `full`: Enables all features
//!
//! ## Usage
//!
//! Add the crate to your dependencies with the features you need:
//!
//! ```toml
//! [dependencies]
//! floxide = { version = "1.0.0", features = ["transform", "event"] }
//! ```

// Re-export core components (enabled by default with the "core" feature)
#[cfg(feature = "core")]
pub use floxide_core::*;

// Re-export transform functionality
#[cfg(feature = "transform")]
pub use floxide_transform::*;

// Re-export event functionality
#[cfg(feature = "event")]
pub use floxide_event::*;

// Re-export timer functionality
#[cfg(feature = "timer")]
pub use floxide_timer::*;

// Re-export longrunning functionality
#[cfg(feature = "longrunning")]
pub use floxide_longrunning::*;

// Re-export reactive functionality
#[cfg(feature = "reactive")]
pub use floxide_reactive::*;

/// Initialize the framework with default settings.
///
/// This sets up tracing for better logging and performs any necessary
/// initialization for the enabled features.
pub fn init() {
    // Initialize tracing for better logs
    tracing_subscriber::fmt::init();

    // Additional initialization based on enabled features
    #[cfg(feature = "core")]
    {
        // Core-specific initialization if needed
    }

    #[cfg(feature = "event")]
    {
        // Event-specific initialization if needed
    }

    // Other feature-specific initialization can be added here
}
