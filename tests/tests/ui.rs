use std::fs;

// covers: R-0792
#[test]
fn ui() {
    // trybuild passes quietly when a pattern matches nothing.
    for dir in ["ui/pass", "ui/fail"] {
        assert!(
            fs::read_dir(dir).unwrap().next().is_some(),
            "{dir} is empty"
        );
    }
    let cases = trybuild::TestCases::new();
    cases.pass("ui/pass/*.rs");
    cases.compile_fail("ui/fail/*.rs");
}
