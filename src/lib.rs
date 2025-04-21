//! # Floxide - A directed graph workflow system in Rust
//!
//! This crate provides a flexible, type-safe framework for building directed graph workflows.
//! It allows you to create complex processing pipelines with clear transitions between steps.
//!
//! ## Features
//!
//! The crate is organized into feature-gated modules:
//!
//! - Core functionality is always included
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

/// Initialize the framework with default settings.
///
/// This sets up tracing for better logging and performs any necessary
/// initialization for the enabled features.
pub fn init() {
    // Initialize tracing for better logs
    tracing_subscriber::fmt::init();
}

// Re-export the core module (always included)
pub use floxide_core as core;

// // Re-export the transform module
// #[cfg(feature = "transform")]
// pub use floxide_transform as transform;

// // Re-export the event module
// #[cfg(feature = "event")]
// pub use floxide_event as event;

// // Re-export the timer module
// #[cfg(feature = "timer")]
// pub use floxide_timer as timer;

// // Re-export the longrunning module
// #[cfg(feature = "longrunning")]
// pub use floxide_longrunning as longrunning;

// // Re-export the reactive module
// #[cfg(feature = "reactive")]
// pub use floxide_reactive as reactive;
