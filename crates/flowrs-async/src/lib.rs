//! # Flowrs Async
//!
//! DEPRECATED: This crate has been renamed to `flowrs-transform`. Please update your imports.
//!
//! This crate is a thin re-export layer that provides backward compatibility while
//! we transition to the new `flowrs-transform` crate. It will be removed in a future version.
//!
//! Please update your imports from `flowrs_async` to `flowrs_transform`.

#![deprecated(
    since = "0.2.0",
    note = "This crate has been renamed to flowrs-transform. Please update your imports from flowrs_async to flowrs_transform."
)]

// Re-export everything from flowrs-transform
pub use flowrs_transform::*;
