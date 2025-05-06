use trybuild::TestCases;

#[test]
// Checks that a type mismatch between connected nodes produces an error mentioning both node names.
fn type_mismatch() {
    let t = TestCases::new();
    t.compile_fail("tests/ui/type_mismatch.rs");
}