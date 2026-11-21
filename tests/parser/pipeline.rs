//! Cross-crate integration tests for the parser's public seam.
//!
//! These live outside `nava-parser` so they exercise only what the driver and
//! later phases can actually call: `nava_parser::parse`.

use nava_parser::parse;

const FIBONACCI: &str = include_str!("../../examples/fibonacci.ts");
const BAD_SYNTAX: &str = include_str!("../../examples/bad_syntax.ts");

#[test]
fn parses_the_fibonacci_fixture() {
    let output = parse(FIBONACCI);
    assert!(
        !output.diagnostics.has_errors(),
        "{}",
        output.diagnostics.render(FIBONACCI)
    );
    assert_eq!(output.program.items.len(), 2);
    let debug = format!("{:?}", output.program);
    assert!(debug.contains("Function"));
}

#[test]
fn reports_a_positioned_syntax_error() {
    let output = parse(BAD_SYNTAX);
    assert!(output.diagnostics.has_errors());
    let rendered = output.diagnostics.render(BAD_SYNTAX);
    assert!(rendered.contains("<input>:1:14: error:"), "{rendered}");
    assert!(rendered.contains('^'), "{rendered}");
}

#[test]
fn keeps_later_items_after_a_syntax_error() {
    let output = parse(BAD_SYNTAX);
    assert_eq!(output.program.items.len(), 1);
}
