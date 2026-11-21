//! Tests for the type checker's public `check` seam.

use nava_ast::{ItemKind, Program};
use nava_parser::parse;
use nava_symbol::Type;
use nava_typechecker::{CheckOutput, check};

fn check_source(source: &str) -> (CheckOutput, Program) {
    let parsed = parse(source);
    assert!(
        !parsed.diagnostics.has_errors(),
        "syntax errors: {}",
        parsed.diagnostics.render(source)
    );
    (check(&parsed.program, source), parsed.program)
}

fn messages(output: &CheckOutput) -> Vec<String> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect()
}

#[test]
fn accepts_inferred_and_annotated_variables() {
    let (output, _) = check_source(
        "let n = 1; let s: string = \"hi\"; let b: boolean = true; let xs: number[] = [1, 2];",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_incompatible_assignment() {
    let (output, _) = check_source("let n: number = \"oops\";");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign 'string' to 'number'"))
    );
}

#[test]
fn rejects_wrong_argument_count_and_type() {
    let (output, _) = check_source(
        "function add(a: number, b: number): number { return a + b; } add(1); add(1, \"two\");",
    );
    let messages = messages(&output);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("expected 2 arguments, found 1"))
    );
    assert!(messages.iter().any(|m| m.contains("expected 'number'")));
}

#[test]
fn rejects_unknown_identifier() {
    let (output, _) = check_source("let x = missing;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("unknown identifier 'missing'"))
    );
}

#[test]
fn allows_shadowing_in_inner_scopes() {
    let (output, _) =
        check_source("let x: number = 1; function f(x: string): string { return x; }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_duplicate_declarations() {
    let (output, _) = check_source("let x = 1; let x = 2;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("duplicate declaration 'x'"))
    );
}

#[test]
fn checks_return_types() {
    let (output, _) = check_source("function f(): number { return \"nope\"; }");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("return type 'string' is not assignable"))
    );
}

#[test]
fn resolves_aliases_and_interfaces() {
    let (output, _) = check_source(
        "type Id = number; interface Point { x: number; } let id: Id = 1; let p: Point = { x: 2 }; let y = p.x;",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_missing_object_property() {
    let (output, _) = check_source("interface Point { x: number; } let p: Point = { y: 1 };");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign"))
    );
}

#[test]
fn infers_fibonacci_without_errors() {
    let source = "function fibonacci(n: number): number { if (n <= 1) { return n; } return fibonacci(n - 1) + fibonacci(n - 2); } let result = fibonacci(10);";
    let (output, program) = check_source(source);
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    assert!(matches!(program.items[0].kind, ItemKind::Function(_)));
}

#[test]
fn stores_expression_types_for_later_phases() {
    let (output, _) = check_source("let n = 1 + 2;");
    assert!(output.typed.types.values().any(|ty| *ty == Type::Number));
}

#[test]
fn infers_return_types_with_parameters_in_scope() {
    let (output, _) = check_source("function f(x: number) { return x; }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn later_functions_are_visible_to_earlier_bodies() {
    let (output, _) = check_source("function a() { return b(); } function b() { return 1; }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_duplicate_local_declarations() {
    let (output, _) = check_source("function f() { let x = 1; let x = 2; }");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("duplicate declaration 'x'"))
    );
}

#[test]
fn checks_property_and_element_assignments() {
    let (output, _) = check_source(
        "interface P { x: number; } let p: P = { x: 1 }; p.x = \"s\"; let a: number[] = [1]; a[0] = \"s\";",
    );
    let messages = messages(&output);
    assert!(
        messages
            .iter()
            .filter(|message| message.contains("cannot assign 'string' to 'number'"))
            .count()
            >= 2
    );
}

#[test]
fn follows_alias_chains_and_reports_cycles() {
    let (output, _) = check_source("type A = B; type B = number; let x: A = 1;");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    let (output, _) = check_source("type Loop = Loop;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("circular type alias 'Loop'"))
    );
}

#[test]
fn supports_string_length_and_indexing() {
    let (output, _) =
        check_source("let s = \"abc\"; let n = s.length; let c = s[0]; let same = s == \"abc\";");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    let typed_string = output.typed.types.values().any(|ty| *ty == Type::String);
    assert!(typed_string);
}

#[test]
fn supports_array_length_and_element_assignment() {
    let (output, _) = check_source("let xs: number[] = [1]; let n = xs.length; xs[0] = 2;");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn checks_loop_scopes_and_for_of_element_types() {
    let (output, _) = check_source(
        "let xs: number[] = [1]; for (let x of xs) { let y: number = x; } for (let i = 0; i < 3; i = i + 1) { let z = i; } while (true) { let w = 1; }",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_for_of_over_a_number() {
    let (output, _) = check_source("for (let x of 1) { let y = x; }");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("for-of requires an array or string"))
    );
}

#[test]
fn infers_arrow_function_types() {
    let (output, _) = check_source(
        "let add = (a: number, b: number): number => a + b; let sq = (x: number) => x * x; let n = add(1, 2);",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn requires_arrow_parameter_annotations() {
    let (output, _) = check_source("let sq = x => x * x;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("arrow function parameters require a type annotation"))
    );
}

#[test]
fn rejects_wrong_arrow_call_arity() {
    let (output, _) = check_source("let add = (a: number, b: number): number => a + b; add(1);");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("expected 2 arguments, found 1"))
    );
}

#[test]
fn checks_compound_updates_power_and_ternary() {
    let (output, _) = check_source(
        "let x = 2; x += 3; x++; --x; let p = x ** 2; let t = x > 0 ? \"yes\" : \"no\"; let s = \"a\"; s += \"b\";",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_non_numeric_update_and_power() {
    let (output, _) = check_source("let s = \"x\"; s++; let b = 2 ** \"a\";");
    let messages = messages(&output);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("++ requires a number"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("arithmetic operator expects numbers"))
    );
}

#[test]
fn checks_assignment_form_for_of() {
    let (output, _) =
        check_source("let x = 0; let xs: number[] = [1]; for (x of xs) { let y = x; }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    let (output, _) = check_source("for (missing of xs) { let y = missing; }");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("unknown identifier 'missing'"))
    );
}

#[test]
fn accepts_an_infinite_loop_that_always_returns() {
    let (output, _) = check_source("function f(n: number): number { while (true) { return n; } }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn infers_returns_inside_loops() {
    let (output, _) =
        check_source("function f(n: number): number { while (n > 0) { return 1; } return 2; }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_comparisons_of_non_orderable_types() {
    let (output, _) = check_source("let x = true < false;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("comparison requires two numbers or two strings"))
    );
}
