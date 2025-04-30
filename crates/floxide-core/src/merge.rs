//! Merge trait for user-defined context merging

use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// A trait representing a merge operation (monoid) over values of the same type.
///
/// Types implementing `Merge` must also implement `Default`, which serves as the identity element.
/// The `merge` method combines `other` into `self`, consuming `other`.
/// Implementations should respect monoid laws:
/// - left identity: `Default::default().merge(x)` yields `x` (up to equivalence)
/// - right identity: `x.merge(Default::default())` yields `x` (up to equivalence)
/// - associativity: `(x.merge(y)).merge(z)` yields the same result as `x.merge(y.merge(z))`.
pub trait Merge: Default {
    /// Merge `other` into `self`, consuming `other`.
    fn merge(&mut self, other: Self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixed<T> {
    value: T,
}

impl<T> Fixed<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Fixed<T> {
    pub fn get(&self) -> &T {
        &self.value
    }
}

impl<T: Default> Merge for Fixed<T> {
    fn merge(&mut self, _other: Self) {
        // No-op
    }
}

impl<T: Default> Default for Fixed<T> {
    fn default() -> Self {
        Self {
            value: Default::default(),
        }
    }
}

// Implement getting reference trait for Fixed
impl<T> Deref for Fixed<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
