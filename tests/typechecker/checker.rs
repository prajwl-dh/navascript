//! Tests for the type checker's public `check` seam.

use navascript_ast::{ItemKind, Program};
use navascript_parser::parse;
use navascript_symbol::Type;
use navascript_typechecker::{CheckOutput, check};

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
fn rejects_break_and_continue_outside_loops() {
    let (output, _) = check_source("break; continue;");
    let messages = messages(&output);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("break outside a loop"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("continue outside a loop"))
    );
}

#[test]
fn accepts_break_and_continue_inside_loops() {
    let (output, _) =
        check_source("for (let i = 0; i < 3; i = i + 1) { if (i == 1) continue; break; }");
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn checks_optional_access() {
    let (output, _) = check_source(
        "interface Box { name: string; } let b: Box = { name: \"x\" }; let n = b?.name; let xs: string[] = [\"a\"]; let v = xs?.[0];",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_optional_access_with_a_scalar_result() {
    let (output, _) =
        check_source("interface Box { n: number; } let b: Box = { n: 1 }; let v = b?.n;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("optional chaining requires a reference-typed result"))
    );
}

#[test]
fn rejects_assignment_through_optional_chaining() {
    let (output, _) = check_source(
        "interface Box { name: string; } let b: Box = { name: \"x\" }; b?.name = \"y\";",
    );
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign through optional chaining"))
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
fn type_checks_function_values_and_higher_order_calls() {
    let (output, _) = check_source(
        "function apply(f: (n: number) => number, n: number): number { return f(n); } let twice = (x: number): number => x * 2; let result = apply(twice, 4);",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_wrong_argument_type_for_a_function_value() {
    let (output, _) = check_source("let f = (x: number): number => x; f(\"wrong\");");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("argument has type")),
        "{:?}",
        messages(&output)
    );
}

#[test]
fn rejects_wrong_argument_type_for_a_function_parameter() {
    let (output, _) = check_source(
        "function apply(f: (n: number) => number, n: number): number { return f(n); } apply(\"not a function\", 1);",
    );
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("argument has type")),
        "{:?}",
        messages(&output)
    );
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
fn infers_bigint_literals_and_arithmetic() {
    let (output, _) = check_source(
        "let a = 123n; let b: bigint = 0xffn; let sum = a + b; let cmp = a > b; let neg = -a; let xs = [1n, 2n]; let boxed = { value: 3n };",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    assert!(
        output.typed.types.values().any(|ty| *ty == Type::BigInt),
        "expected inferred bigint types"
    );
}

#[test]
fn rejects_mixing_bigint_and_number() {
    let (output, _) = check_source("let bad = 1n + 2;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot mix bigint with other types"))
    );
}

#[test]
fn rejects_bigint_assigned_to_number() {
    let (output, _) = check_source("let n: number = 1n;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign 'bigint' to 'number'"))
    );
}

#[test]
fn rejects_bigint_with_logical_operators() {
    let (output, _) = check_source("let bad = 1n && 2n;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("logical operators require booleans"))
    );
}

#[test]
fn checks_optional_calls_like_direct_calls() {
    let (output, _) = check_source(
        "function f(a: number): number { return a; } let ok = f?.(1); let wrong = f?.();",
    );
    let messages = messages(&output);
    assert!(
        messages
            .iter()
            .any(|message| message.contains("expected 1 arguments, found 0")),
        "{messages:?}"
    );
}

#[test]
fn checks_exported_declarations_and_imported_names() {
    let (output, _) = check_source(
        "import { f } from \"./m.ns\"; export function f(): number { return 1; } console.log(f());",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    let (output, _) = check_source("import { missing } from \"./m.ns\";");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("imported name 'missing' is not exported")),
        "{:?}",
        messages(&output)
    );
    let (output, _) = check_source(
        "export class C {} export interface I {} export type T = number; import { C, I, T } from \"./m.ns\";",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
    let (output, _) = check_source("const hidden = 1; import { hidden } from \"./m.ns\";");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("imported name 'hidden' is not exported"))
    );
}

#[test]
fn accepts_union_and_intersection_assignments() {
    let (output, _) = check_source(
        "interface A { a: number; } interface B { b: string; } let u: string | null = \"x\"; let v: string | null = null; let i: A & B = { a: 1, b: \"x\" };",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn accepts_union_assignment_in_either_member_order() {
    let (output, _) = check_source(
        "let a: string | null = \"x\"; let b: null | string = a; let c: string | null = b;",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_unions_with_mixed_runtime_representations() {
    let (output, _) = check_source("let u: number | string = 1;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("union members must share a runtime representation")),
        "{:?}",
        messages(&output)
    );
    let (output, _) = check_source("let n: number | null = 1;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("nullable unions require a reference type"))
    );
}

#[test]
fn rejects_values_outside_a_union() {
    let (output, _) = check_source("let u: number | string = true;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign")),
        "{:?}",
        messages(&output)
    );
}

#[test]
fn narrows_nullable_unions_in_conditions() {
    let (output, _) = check_source(
        "function f(x: string | null): number { if (x != null) { return x.length; } return 0; }",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn narrows_in_logical_conditions_and_loops() {
    let (output, _) = check_source(
        "function f(x: string | null): void { if (x != null && x.length > 0) { console.log(x.length); } while (x != null) { console.log(x.length); break; } }",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn accepts_missing_optional_properties() {
    let (output, _) = check_source(
        "interface P { name: string; nickname?: string; } let a: P = { name: \"x\" }; let b: P = { name: \"y\", nickname: \"z\" };",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn resolves_intersection_members() {
    let (output, _) = check_source(
        "interface A { a: number; } interface B { b: string; } let both: A & B = { a: 1, b: \"x\" }; console.log(both.a); console.log(both.b);",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn checks_classes_fields_and_methods() {
    let (output, _) = check_source(
        "class Point { x: number; constructor(x: number) { this.x = x; } sum(): number { return this.x; } } let p = new Point(2); console.log(p.sum());",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn inherits_fields_and_methods() {
    let (output, _) = check_source(
        "class A { n: number; constructor(n: number) { this.n = n; } value(): number { return this.n; } } class B extends A { constructor(n: number) { this.n = n; } } let b = new B(3); console.log(b.value());",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn resolves_super_method_calls() {
    let (output, _) = check_source(
        "class A { n: number; constructor(n: number) { this.n = n; } value(): number { return this.n; } } class B extends A { constructor(n: number) { this.n = n; } value(): number { return super.value() + 1; } } let b = new B(4); console.log(b.value());",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn inherits_constructors_and_accepts_super_calls() {
    let (output, _) = check_source(
        "class A { n: number; constructor(n: number) { this.n = n; } } class B extends A { constructor(n: number) { super(n); } } class C extends A {} new B(1); new C(2);",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_bad_super_call_arity() {
    let (output, _) = check_source(
        "class A { n: number; constructor(n: number) { this.n = n; } } class B extends A { constructor() { super(); } }",
    );
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("expected 1 base constructor arguments, found 0"))
    );
}

#[test]
fn rejects_unknown_classes_and_bad_constructor_calls() {
    let (output, _) = check_source("let x = new Missing(1);");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("unknown class 'Missing'"))
    );
    let (output, _) =
        check_source("class A { n: number; constructor(n: number) { this.n = n; } } new A();");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("expected 1 constructor arguments, found 0"))
    );
}

#[test]
fn rejects_unknown_methods_and_this_outside_a_class() {
    let (output, _) = check_source(
        "class A { n: number; constructor(n: number) { this.n = n; } } let a = new A(1); a.missing();",
    );
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("property 'missing' does not exist")),
        "{:?}",
        messages(&output)
    );
    let (output, _) = check_source("let x = this;");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("'this' is only valid inside a class"))
    );
}

#[test]
fn infers_generic_type_arguments_from_arguments() {
    let (output, _) = check_source(
        "function id<T>(value: T): T { return value; } let n = id(1); let s = id(\"x\");",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn accepts_explicit_and_multiple_type_arguments() {
    let (output, _) = check_source(
        "function id<T>(value: T): T { return value; } let b = id<boolean>(true); function pair<A, B>(a: A, b: B): A { return a; } let n = pair<number, string>(1, \"x\"); let m = pair(2, \"y\");",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn instantiates_generic_interfaces() {
    let (output, _) = check_source(
        "interface Box<T> { value: T; } let n: Box<number> = { value: 1 }; let s: Box<string> = { value: \"x\" }; console.log(n.value);",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_unknown_generic_type_references() {
    let (output, _) =
        check_source("interface Box<T> { value: T; } let bad: Unknown<T> = { value: 1 };");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("unknown generic type 'Unknown'")),
        "{:?}",
        messages(&output)
    );
}

#[test]
fn rejects_wrong_generic_annotation_arity() {
    let (output, _) =
        check_source("interface Box<T> { value: T; } let bad: Box<number, string> = { value: 1 };");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("expected 1 type arguments for 'Box', found 2")),
        "{:?}",
        messages(&output)
    );
}

#[test]
fn rejects_wrong_generic_type_argument_count() {
    let (output, _) =
        check_source("function id<T>(value: T): T { return value; } id<number, string>(1);");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("expected 1 type arguments, found 2"))
    );
}

#[test]
fn rejects_calls_whose_type_parameter_cannot_be_inferred() {
    let (output, _) = check_source("function make<T>(): T { return 1; } let x = make();");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot infer type parameter 'T'"))
    );
}

#[test]
fn rejects_generic_argument_type_mismatches() {
    let (output, _) =
        check_source("function id<T>(value: T): T { return value; } id<number>(\"x\");");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("argument has type 'string', expected 'number'"))
    );
}

#[test]
fn rejects_assigning_a_mismatched_generic_interface() {
    let (output, _) =
        check_source("interface Box<T> { value: T; } let b: Box<string> = { value: 1 };");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign"))
    );
}

#[test]
fn rejects_calling_a_non_callable_with_optional_syntax() {
    let (output, _) = check_source("let n = 1; n?.();");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot call a value of type 'number'"))
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
fn checks_throw_against_the_catch_type() {
    let (output, _) = check_source(
        "try { throw \"bad\"; } catch (e: string) { console.log(e); } finally { console.log(\"done\"); }",
    );
    assert!(!output.diagnostics.has_errors(), "{:?}", messages(&output));
}

#[test]
fn rejects_unsupported_exception_forms() {
    let (output, _) = check_source("throw \"outside\";");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("requires an enclosing try"))
    );
    let (output, _) = check_source("try { throw 1; } catch (e: string) {}");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cannot assign 'number' to 'string'"))
    );
    let (output, _) =
        check_source("function f(): number { try { return 1; } catch (e: string) { return 2; } }");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("return inside try/catch/finally"))
    );
    let (output, _) = check_source("let g = (): void => { throw \"arrow\"; };");
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("cross-function unwinding"))
    );
}

#[test]
fn rejects_break_continue_and_nested_throw_in_protected_regions() {
    let (output, _) =
        check_source("while (true) { try { break; continue; } catch (e: string) {} }");
    let found = messages(&output);
    assert!(
        found
            .iter()
            .any(|message| message.contains("break inside try/catch/finally"))
    );
    assert!(
        found
            .iter()
            .any(|message| message.contains("continue inside try/catch/finally"))
    );
    let (output, _) = check_source(
        "try { throw \"x\"; } catch (e: string) { try { throw \"y\"; } catch (f: string) {} finally { console.log(\"done\"); } }",
    );
    assert!(
        messages(&output)
            .iter()
            .any(|message| message.contains("throw inside catch/finally"))
    );
}

#[test]
fn accepts_alias_catch_types() {
    let (output, _) = check_source(
        "type Failure = string; try { throw \"bad\"; } catch (e: Failure) { console.log(e); }",
    );
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
