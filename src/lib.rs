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


// Re-export the core module (always included)
pub use floxide_core::*;
pub use ::floxide_macros::*;

