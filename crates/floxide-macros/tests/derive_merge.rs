// Integration test for the `Merge` derive macro
// This test is compiled as a separate crate and verifies that deriving Merge works end-to-end.

// Bring the proc-macro derive into scope (crate name is floxide_macros)
extern crate floxide_macros;
// Merge trait is defined in the core crate
extern crate floxide_core;

use floxide_core::Merge;
use floxide_macros::Merge;


// Define a test struct with two fields that implement Merge
#[derive(Default, Merge)]
struct SimpleCtx {
    a: i32,
    b: Vec<i32>,
}

#[test]
fn test_merge_simple_ctx() {
    let mut x = SimpleCtx { a: 2, b: vec![10] };
    let y = SimpleCtx { a: 3, b: vec![20, 30] };
    x.merge(y);
    assert_eq!(x.a, 5);
    assert_eq!(x.b, vec![10, 20, 30]);
}