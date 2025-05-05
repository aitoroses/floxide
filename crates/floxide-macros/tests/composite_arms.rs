use syn::parse_str;
use floxide_macros_support::{WorkflowDef, EdgeKind};

fn parse_edges(input: &str) -> Vec<(syn::Ident, EdgeKind)> {
    // Provide a minimal valid workflow definition for parsing
    let src = format!(r#"
        pub struct Dummy {{ foo: usize }}
        start = foo;
        edges {{ {} }}
    "#, input);
    let def: WorkflowDef = parse_str(&src).unwrap();
    def.edges
}

#[test]
fn parses_unit_variant() {
    let edges = parse_edges("foo => { MyEnum::Done => [bar]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].variant, "Done");
        assert!(arms[0].binding.is_none());
        assert!(arms[0].is_wildcard);
        assert!(arms[0].guard.is_none());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_binding_variant() {
    let edges = parse_edges("foo => { MyEnum::Valid(data) => [bar]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].variant, "Valid");
        assert_eq!(arms[0].binding.as_ref().unwrap(), "data");
        assert!(!arms[0].is_wildcard);
        assert!(arms[0].guard.is_none());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_wildcard_variant() {
    let edges = parse_edges("foo => { MyEnum::Valid(_) => [bar]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].variant, "Valid");
        assert!(arms[0].binding.is_none());
        assert!(arms[0].is_wildcard);
        assert!(arms[0].guard.is_none());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_unit_variant_with_guard() {
    let edges = parse_edges("foo => { MyEnum::Done if some_check() => [bar]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].variant, "Done");
        assert!(arms[0].binding.is_none());
        assert!(arms[0].is_wildcard);
        assert!(arms[0].guard.is_some());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_binding_with_guard() {
    let edges = parse_edges("foo => { MyEnum::Valid(data) if data.is_ok() => [bar]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].variant, "Valid");
        assert_eq!(arms[0].binding.as_ref().unwrap(), "data");
        assert!(!arms[0].is_wildcard);
        assert!(arms[0].guard.is_some());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_wildcard_with_guard() {
    let edges = parse_edges("foo => { MyEnum::Valid(_) if some_check() => [bar]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].variant, "Valid");
        assert!(arms[0].binding.is_none());
        assert!(arms[0].is_wildcard);
        assert!(arms[0].guard.is_some());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_multiple_arms() {
    let edges = parse_edges("foo => { MyEnum::Valid(data) => [next_node]; MyEnum::Invalid(_) => [error_node]; MyEnum::Done => [finish_node]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms.len(), 3);
        assert_eq!(arms[0].variant, "Valid");
        assert_eq!(arms[1].variant, "Invalid");
        assert_eq!(arms[2].variant, "Done");
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_multiple_successors() {
    let edges = parse_edges("foo => { MyEnum::Valid(data) => [a, b, c]; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert_eq!(arms[0].succs.len(), 3);
        assert_eq!(arms[0].succs[0], "a");
        assert_eq!(arms[0].succs[1], "b");
        assert_eq!(arms[0].succs[2], "c");
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_terminal_composite_arm() {
    let edges = parse_edges("foo => { MyEnum::Done => []; };");
    if let EdgeKind::Composite(arms) = &edges[0].1 {
        assert!(arms[0].succs.is_empty());
    } else {
        panic!("Expected composite edge");
    }
}

#[test]
fn parses_direct_edge() {
    let edges = parse_edges("foo => [bar];");
    if let EdgeKind::Direct { succs, on_failure } = &edges[0].1 {
        assert_eq!(succs[0], "bar");
        assert!(on_failure.is_none());
    } else {
        panic!("Expected direct edge");
    }
} 