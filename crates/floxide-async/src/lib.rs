//! # Floxide Async
//!
//! DEPRECATED: This crate has been renamed to `floxide-transform`. Please update your imports.
//!
//! This crate is a thin re-export layer that provides backward compatibility while
//! we transition to the new `floxide-transform` crate. It will be removed in a future version.
//!
//! Please update your imports from `floxide_async` to `floxide_transform`.

#![deprecated(
    since = "0.2.0",
    note = "This crate has been renamed to floxide-transform. Please update your imports from floxide_async to floxide_transform."
)]

// Re-export everything from floxide-transform
pub use floxide_transform::*;
