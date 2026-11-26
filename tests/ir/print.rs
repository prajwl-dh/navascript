//! Tests for textual IR output.

use navascript_ir::{lower, print};
use navascript_parser::parse;
use navascript_typechecker::check;

#[test]
fn prints_the_fibonacci_module() {
    let source = include_str!("../../examples/fibonacci.ns");
    let parsed = parse(source);
    let typed = check(&parsed.program, source);
    let lowered = lower(&parsed.program, &typed.typed, source);
    let text = print(&lowered.module);

    assert!(
        text.contains("fn fibonacci(%0: number) -> number {"),
        "{text}"
    );
    assert!(text.contains("bb0 entry:"), "{text}");
    assert!(text.contains("$0 = alloca number"), "{text}");
    assert!(text.contains("br %2 ? bb1 : bb2"), "{text}");
    assert!(text.contains("call fibonacci("), "{text}");
    assert!(text.contains("call console.log("), "{text}");
    assert!(text.contains("fn __navascript_entry() -> void {"), "{text}");
}

#[test]
fn prints_string_globals() {
    let source = "console.log(\"hello\");";
    let parsed = parse(source);
    let typed = check(&parsed.program, source);
    let lowered = lower(&parsed.program, &typed.typed, source);
    let text = print(&lowered.module);
    assert!(text.contains("global @0 = \"hello\""), "{text}");
}
