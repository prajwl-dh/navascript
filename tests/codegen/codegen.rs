//! Tests for Nava IR → LLVM IR translation.

use nava_codegen::generate;
use nava_ir::lower;
use nava_parser::parse;
use nava_typechecker::check;

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
        ir.contains("declare void @nava_console_log_f64(double)"),
        "{ir}"
    );
    assert!(
        ir.contains("declare void @nava_console_log_bool(i1)"),
        "{ir}"
    );
}

#[test]
fn emits_functions_with_mangled_names() {
    let source = include_str!("../../examples/fibonacci.ts");
    let ir = generate_source(source);
    assert!(
        ir.contains("define double @nava_fibonacci(double %p0)"),
        "{ir}"
    );
    assert!(ir.contains("define void @nava_main()"), "{ir}");
    assert!(ir.contains("call double @nava_fibonacci(double"), "{ir}");
    assert!(
        ir.contains("call void @nava_console_log_f64(double"),
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
    assert!(ir.contains("define void @nava_main()"), "{ir}");
}

#[test]
fn emits_string_literals_as_llvm_globals() {
    let ir = generate_source("console.log(\"hello\");");
    assert!(ir.contains("c\"hello\\00\""), "{ir}");
    assert!(
        ir.contains("call void @nava_console_log_str(ptr @g0)"),
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
    assert!(ir.contains("call ptr @nava_array_new(i64 2)"), "{ir}");
    assert!(ir.contains("call double @nava_array_get_f64"), "{ir}");
    assert!(ir.contains("call ptr @nava_object_new(i64 1"), "{ir}");
    assert!(ir.contains("call double @nava_object_get_f64"), "{ir}");
}
