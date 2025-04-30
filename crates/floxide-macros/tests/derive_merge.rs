// Integration test for the `Merge` derive macro
// This test is compiled as a separate crate and verifies that deriving Merge works end-to-end.

// Bring the proc-macro derive into scope (crate name is floxide_macros)
extern crate floxide_macros;
// Merge trait is defined in the core crate
extern crate floxide_core;

use floxide_core::Merge;
use floxide_macros::Merge;

pub struct Counter {
    count: i32,
}

impl Default for Counter {
    fn default() -> Self {
        Self { count: 0 }
    }
}

impl Merge for Counter {
    fn merge(&mut self, other: Self) {
        self.count += other.count;
    }
}

pub struct Log {
    messages: Vec<String>,
}

impl Default for Log {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl Merge for Log {
    fn merge(&mut self, other: Self) {
        self.messages.extend(other.messages);
    }
}

// Define a test struct with two fields that implement Merge
#[derive(Default, Merge)]
struct SimpleCtx {
    a: Counter,
    b: Log,
}

#[test]
fn test_merge_simple_ctx() {
    let mut x = SimpleCtx {
        a: Counter { count: 2 },
        b: Log {
            messages: vec!["a".to_string()],
        },
    };
    let y = SimpleCtx {
        a: Counter { count: 3 },
        b: Log {
            messages: vec!["b".to_string(), "c".to_string()],
        },
    };
    x.merge(y);
    assert_eq!(x.a.count, 5);
    assert_eq!(x.b.messages, vec!["a", "b", "c"]);
}
