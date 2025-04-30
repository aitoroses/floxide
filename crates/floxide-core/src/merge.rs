//! Merge trait for user-defined context merging
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

impl Merge for i32 {
    fn merge(&mut self, other: Self) {
        *self += other;
    }
}

impl<T> Merge for Vec<T> {
    fn merge(&mut self, mut other: Self) {
        self.append(&mut other);
    }
}