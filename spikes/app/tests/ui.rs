//! The compile errors the proofs need, with their messages pinned in `.stderr`.
//! Compiler output depends on the toolchain: run on 1.99.1.

#[test]
fn compile_errors() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
