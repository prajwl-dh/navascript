//! Tests for the parser's public `parse` seam.

use nava_ast::{BinaryOp, ElseBranch, ExprKind, FileId, ItemKind, Span, StmtKind, TypeExpr};
use nava_diagnostics::{Diagnostic, Diagnostics, Severity};
use nava_parser::{ParseOutput, parse};

fn ok(source: &str) -> ParseOutput {
    let output = parse(source);
    assert!(
        !output.diagnostics.has_errors(),
        "unexpected diagnostics: {}",
        output.diagnostics.render(source)
    );
    output
}

#[test]
fn parses_variable_with_inference() {
    let output = ok("let answer = 42;");
    assert_eq!(output.program.items.len(), 1);
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert_eq!(decl.name.name, "answer");
    assert!(decl.ty.is_none());
    assert!(matches!(decl.init.as_ref().unwrap().kind, ExprKind::Number(n) if n == 42.0));
}

#[test]
fn parses_typed_variable() {
    let output = ok("const name: string = \"nava\";");
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &decl.ty.as_ref().unwrap().expr,
        TypeExpr::Named(name) if name == "string"
    ));
    assert!(matches!(&decl.init.as_ref().unwrap().kind, ExprKind::String(s) if s == "nava"));
}

#[test]
fn parses_function_with_parameters_and_return_type() {
    let output = ok("function add(a: number, b: number): number { return a + b; }");
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    assert_eq!(decl.name.name, "add");
    assert_eq!(decl.params.len(), 2);
    assert!(matches!(
        &decl.return_ty.as_ref().unwrap().expr,
        TypeExpr::Named(name) if name == "number"
    ));
    assert_eq!(decl.body.statements.len(), 1);
    assert!(matches!(decl.body.statements[0].kind, StmtKind::Return(_)));
}

#[test]
fn respects_operator_precedence() {
    let output = ok("let x = 1 + 2 * 3;");
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    let ExprKind::Binary { op, right, .. } = &decl.init.as_ref().unwrap().kind else {
        panic!("expected a binary expression");
    };
    assert_eq!(*op, BinaryOp::Add);
    assert!(matches!(
        &right.kind,
        ExprKind::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn parses_calls_and_member_access() {
    let output = ok("console.log(1);");
    let ItemKind::Expr(expr) = &output.program.items[0].kind else {
        panic!("expected an expression item");
    };
    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected a call");
    };
    assert!(matches!(&callee.kind, ExprKind::Member { property, .. } if property == "log"));
    assert_eq!(args.len(), 1);
}

#[test]
fn parses_if_else_chain() {
    let output =
        ok("function f() { if (a) { return 1; } else if (b) { return 2; } else { return 3; } }");
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    let StmtKind::If(if_stmt) = &decl.body.statements[0].kind else {
        panic!("expected an if statement");
    };
    let Some(ElseBranch::If(second)) = &if_stmt.else_branch else {
        panic!("expected an else-if branch");
    };
    assert!(matches!(second.else_branch, Some(ElseBranch::Block(_))));
}

#[test]
fn parses_the_fibonacci_acceptance_program() {
    let source = "function fibonacci(n: number): number { if (n <= 1) { return n; } return fibonacci(n - 1) + fibonacci(n - 2); } console.log(fibonacci(10));";
    let output = ok(source);
    assert_eq!(output.program.items.len(), 2);
    assert!(matches!(
        output.program.items[0].kind,
        ItemKind::Function(_)
    ));
    assert!(matches!(output.program.items[1].kind, ItemKind::Expr(_)));
}

#[test]
fn reports_line_and_column_and_renders_caret() {
    let source = "let x = ;\nlet y = 1;";
    let output = parse(source);
    assert!(output.diagnostics.has_errors());
    let rendered = output.diagnostics.render(source);
    assert!(rendered.contains("<input>:1:9: error:"), "{rendered}");
    assert!(rendered.contains("^"), "{rendered}");
}

#[test]
fn diagnostics_are_sorted_and_deduplicated() {
    let source = "let x = ;\nlet y = ;";
    let output = parse(source);
    let lines: Vec<u32> = output.diagnostics.iter().map(|d| d.line).collect();
    assert!(
        lines.windows(2).all(|pair| pair[0] <= pair[1]),
        "diagnostics out of order: {lines:?}"
    );
}

#[test]
fn recovers_and_keeps_later_items() {
    let output = parse("let broken = ;\nlet good = 1;");
    assert!(output.diagnostics.has_errors());
    assert_eq!(output.program.items.len(), 1);
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert_eq!(decl.name.name, "good");
}

#[test]
fn handles_empty_input() {
    let output = parse("");
    assert!(output.diagnostics.is_empty());
    assert!(output.program.items.is_empty());
}

#[test]
fn parses_unbraced_if_and_else_bodies() {
    let output = ok("function f() { if (a) return 1; else return 2; }");
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    let StmtKind::If(if_stmt) = &decl.body.statements[0].kind else {
        panic!("expected an if statement");
    };
    assert_eq!(if_stmt.then_branch.statements.len(), 1);
    let Some(ElseBranch::Block(branch)) = &if_stmt.else_branch else {
        panic!("expected an else block");
    };
    assert_eq!(branch.statements.len(), 1);
}

#[test]
fn deep_nesting_is_diagnosed_instead_of_overflowing() {
    let source = format!("let x = {}1{};", "(".repeat(300), ")".repeat(300));
    let output = parse(&source);
    assert!(
        output
            .diagnostics
            .iter()
            .any(|d| d.message.contains("nesting is too deep"))
    );
    let source = format!("let x = {}1;", "!".repeat(300));
    let output = parse(&source);
    assert!(output.diagnostics.has_errors());
}

#[test]
fn diagnostics_deduplicate_and_break_ties_by_severity() {
    let span = Span::new(FileId(0), 1, 2);
    let error = Diagnostic {
        severity: Severity::Error,
        message: "same".to_owned(),
        span,
        line: 1,
        column: 2,
    };
    let warning = Diagnostic {
        severity: Severity::Warning,
        message: "same".to_owned(),
        span,
        line: 1,
        column: 2,
    };
    let diagnostics = Diagnostics::new(vec![warning.clone(), error.clone(), error.clone()]);
    let items: Vec<&Diagnostic> = diagnostics.iter().collect();
    assert_eq!(items.len(), 2, "duplicate was not removed");
    assert_eq!(
        items[0].severity,
        Severity::Error,
        "tie-break is not stable"
    );
    assert_eq!(items[1].severity, Severity::Warning);
}

#[test]
fn program_has_a_debug_form() {
    let output = ok("let x = 1;");
    let debug = format!("{:?}", output.program);
    assert!(debug.contains("Variable"), "{debug}");
    assert!(debug.contains("Number"), "{debug}");
}

#[test]
fn parses_type_alias_and_interface() {
    let source = "type Id = number; interface Point { x: number; y: number; }";
    let output = ok(source);
    assert!(matches!(
        output.program.items[0].kind,
        ItemKind::TypeAlias(_)
    ));
    let ItemKind::Interface(decl) = &output.program.items[1].kind else {
        panic!("expected an interface");
    };
    assert_eq!(decl.name.name, "Point");
    assert_eq!(decl.fields.len(), 2);
}

#[test]
fn parses_array_and_object_types_and_literals() {
    let source = "let xs: number[] = [1, 2]; let p: { x: number } = { x: 1 }; let y = xs[0];";
    let output = ok(source);
    assert_eq!(output.program.items.len(), 3);
    let ItemKind::Variable(first) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        first.ty.as_ref().unwrap().expr,
        TypeExpr::Array(_)
    ));
    assert!(matches!(
        first.init.as_ref().unwrap().kind,
        ExprKind::ArrayLit(_)
    ));
    let ItemKind::Variable(second) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        second.init.as_ref().unwrap().kind,
        ExprKind::ObjectLit(_)
    ));
    let ItemKind::Variable(third) = &output.program.items[2].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        third.init.as_ref().unwrap().kind,
        ExprKind::Index { .. }
    ));
}

#[test]
fn parses_function_type_annotations() {
    let output = ok("let f: (n: number) => number;");
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &decl.ty.as_ref().unwrap().expr,
        TypeExpr::Function { .. }
    ));
}
