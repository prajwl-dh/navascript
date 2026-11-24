//! Cross-crate integration tests for the Phase 2 pipeline: parse → check.
//!
//! Both crates are used only through their public seams, so these tests
//! describe the contract the driver and Phase 3 IR lowering rely on.

use navascript_parser::parse;
use navascript_typechecker::check;

const FIBONACCI: &str = include_str!("../../examples/fibonacci.ns");
const TYPE_ERROR: &str = include_str!("../../examples/type_error.ns");

#[test]
fn type_checks_the_fibonacci_fixture() {
    let parsed = parse(FIBONACCI);
    assert!(!parsed.diagnostics.has_errors());
    let checked = check(&parsed.program, FIBONACCI);
    assert!(
        !checked.diagnostics.has_errors(),
        "{}",
        checked.diagnostics.render(FIBONACCI)
    );
}

#[test]
fn rejects_the_type_error_fixture() {
    let parsed = parse(TYPE_ERROR);
    assert!(!parsed.diagnostics.has_errors());
    let checked = check(&parsed.program, TYPE_ERROR);
    assert!(checked.diagnostics.has_errors());
    let rendered = checked.diagnostics.render(TYPE_ERROR);
    assert!(rendered.contains("cannot assign"), "{rendered}");
    assert!(rendered.contains("<input>:1:21:"), "{rendered}");
}

#[test]
fn exposes_expression_types_for_later_phases() {
    let parsed = parse(FIBONACCI);
    let checked = check(&parsed.program, FIBONACCI);
    assert!(
        !checked.typed.types.is_empty(),
        "IR lowering needs expression types"
    );
    assert!(checked.typed.symbols.lookup("fibonacci").is_some());
}
