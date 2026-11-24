//! Tests for NavaScript IR → LLVM IR translation.

use navascript_codegen::generate;
use navascript_ir::lower;
use navascript_parser::parse;
use navascript_typechecker::check;

const TRIPLE: &str = "arm64-apple-darwin23.0.0";

fn generate_source(source: &str) -> String {
    let parsed = parse(source);
    assert!(!parsed.diagnostics.has_errors());
    let typed = check(&parsed.program, source);
    assert!(
        !typed.diagnostics.has_errors(),
        "{}",
        typed.diagnostics.render(source)
    );
    let lowered = lower(&parsed.program, &typed.typed, source);
    assert!(
        !lowered.diagnostics.has_errors(),
        "{}",
        lowered.diagnostics.render(source)
    );
    generate(&lowered.module, TRIPLE).expect("code generation")
}

#[test]
fn emits_target_triple_and_runtime_declaration() {
    let ir = generate_source("let x = 1;");
    assert!(
        ir.contains("target triple = \"arm64-apple-darwin23.0.0\""),
        "{ir}"
    );
    assert!(
        ir.contains("declare void @navascript_console_log_f64(double)"),
        "{ir}"
    );
    assert!(
        ir.contains("declare void @navascript_console_log_bool(i1)"),
        "{ir}"
    );
}

#[test]
fn emits_functions_with_mangled_names() {
    let source = include_str!("../../examples/fibonacci.ns");
    let ir = generate_source(source);
    assert!(
        ir.contains("define double @navascript_fibonacci(double %p0)"),
        "{ir}"
    );
    assert!(ir.contains("define void @navascript_main()"), "{ir}");
    assert!(
        ir.contains("call double @navascript_fibonacci(double"),
        "{ir}"
    );
    assert!(
        ir.contains("call void @navascript_console_log_f64(double"),
        "{ir}"
    );
}

#[test]
fn emits_floating_point_arithmetic_and_branches() {
    let ir =
        generate_source("function f(n: number): number { if (n <= 1) return n; return n - 1; }");
    assert!(ir.contains("fcmp ole double"), "{ir}");
    assert!(ir.contains("fsub double"), "{ir}");
    assert!(ir.contains("br i1"), "{ir}");
}

#[test]
fn emits_a_main_stub_when_the_module_has_no_top_level_items() {
    let ir = generate_source("function f(): number { return 1; }");
    assert!(ir.contains("define void @navascript_main()"), "{ir}");
}

#[test]
fn emits_string_literals_as_llvm_globals() {
    let ir = generate_source("console.log(\"hello\");");
    assert!(ir.contains("c\"hello\\00\""), "{ir}");
    assert!(
        ir.contains("call void @navascript_console_log_str(ptr @g0)"),
        "{ir}"
    );
}

#[test]
fn emits_llvm_pow_for_exponentiation() {
    let ir = generate_source("let p = 2 ** 3;");
    assert!(
        ir.contains("declare double @llvm.pow.f64(double, double)"),
        "{ir}"
    );
    assert!(ir.contains("call double @llvm.pow.f64(double"), "{ir}");
}

#[test]
fn emits_back_edges_for_loops() {
    let ir = generate_source("let i = 0; while (i < 3) { i = i + 1; }");
    let back_edges = ir.matches("br label %bb1").count();
    assert!(back_edges >= 2, "expected entry and back edge to bb1: {ir}");
}

#[test]
fn emits_runtime_calls_for_collections() {
    let ir = generate_source(
        "let xs: number[] = [1, 2]; console.log(xs[0]); let p = { x: 1 }; console.log(p.x);",
    );
    assert!(ir.contains("call ptr @navascript_array_new(i64 2)"), "{ir}");
    assert!(ir.contains("call double @navascript_array_get_f64"), "{ir}");
    assert!(ir.contains("call ptr @navascript_object_new(i64 1"), "{ir}");
    assert!(
        ir.contains("call double @navascript_object_get_f64"),
        "{ir}"
    );
}

#[test]
fn emits_bigint_constants_and_integer_arithmetic() {
    let ir = generate_source("console.log(123n + 1n); console.log(2n ** 8n);");
    assert!(
        ir.contains("declare void @navascript_console_log_i64(i64)"),
        "{ir}"
    );
    assert!(ir.contains("add i64 123, 1"), "{ir}");
    assert!(
        ir.contains("call i64 @navascript_bigint_pow(i64 2, i64 8)"),
        "{ir}"
    );
    assert!(
        ir.contains("call void @navascript_console_log_i64(i64"),
        "{ir}"
    );
}

#[test]
fn emits_bigint_collection_slots() {
    let ir =
        generate_source("let xs = [1n]; console.log(xs[0]); let p = { v: 2n }; console.log(p.v);");
    assert!(ir.contains("call void @navascript_array_set_i64"), "{ir}");
    assert!(ir.contains("call i64 @navascript_array_get_i64"), "{ir}");
    assert!(ir.contains("call void @navascript_object_set_i64"), "{ir}");
    assert!(ir.contains("call i64 @navascript_object_get_i64"), "{ir}");
}

#[test]
fn emits_pointer_null_comparisons_for_nullable_refs() {
    let ir = generate_source(
        "function f(x: string | null): number { if (x != null) { return x.length; } return 0; }",
    );
    assert!(ir.contains("icmp eq ptr"), "{ir}");
    assert!(ir.contains("xor i1"), "{ir}");
}

#[test]
fn emits_class_methods_as_named_functions() {
    let ir = generate_source(
        "class A { n: number; constructor(n: number) { this.n = n; } value(): number { return this.n; } } let a = new A(2); console.log(a.value());",
    );
    assert!(
        ir.contains("define void @navascript_A__ctor(ptr %p0, double %p1)"),
        "{ir}"
    );
    assert!(
        ir.contains("define double @navascript_A_value(ptr %p0)"),
        "{ir}"
    );
    assert!(ir.contains("call double @navascript_A_value(ptr"), "{ir}");
}

#[test]
fn emits_monomorphized_generic_functions() {
    let ir = generate_source(
        "function id<T>(value: T): T { return value; } console.log(id(1)); console.log(id(\"s\"));",
    );
    assert!(
        ir.contains("define double @navascript_id__number(double %p0)"),
        "{ir}"
    );
    assert!(
        ir.contains("define ptr @navascript_id__string(ptr %p0)"),
        "{ir}"
    );
    assert!(
        ir.contains("call double @navascript_id__number(double"),
        "{ir}"
    );
    assert!(ir.contains("call ptr @navascript_id__string(ptr"), "{ir}");
}

#[test]
fn emits_promise_objects_for_async_functions_and_await() {
    let ir = generate_source(
        "async function double(n: number): number { return n * 2; } console.log(await double(3));",
    );
    assert!(
        ir.contains("define ptr @navascript_double(double %p0)"),
        "{ir}"
    );
    assert!(ir.contains("navascript_object_new"), "{ir}");
    assert!(ir.contains("navascript_object_set_f64"), "{ir}");
    assert!(ir.contains("navascript_object_get_f64"), "{ir}");
}

#[test]
fn emits_indirect_calls_for_function_values() {
    let ir = generate_source(
        "function apply(f: (n: number) => number, n: number): number { return f(n); } let g = (x: number): number => x + 1; console.log(apply(g, 4));",
    );
    assert!(ir.contains("bitcast ptr %code"), "{ir}");
    assert!(ir.contains("call double %fpcode"), "{ir}");
    assert!(ir.contains("call void @navascript_object_set_ptr"), "{ir}");
    assert!(ir.contains("define double @navascript_arrow_0("), "{ir}");
}

#[test]
fn emits_optional_calls_as_direct_calls() {
    let ir = generate_source("function f(x: number): number { return x; } console.log(f?.(1));");
    assert!(ir.contains("call double @navascript_f(double"), "{ir}");
}
