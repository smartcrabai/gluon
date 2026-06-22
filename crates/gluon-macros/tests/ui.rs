//! UI tests for the gluon-macros proc-macros.
//!
//! Only `pass` cases are exercised here; `compile_fail` snapshots would
//! require committed `.stderr` files that are brittle across rustc versions.
//! The behavioral failure cases are covered by unit tests in `src/lib.rs`.

#[test]
fn ui_pass() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}
