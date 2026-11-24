//! Tests for the parser's public `parse` seam.

use nava_ast::{
    AssignOp, BinaryOp, ElseBranch, ExprKind, FileId, ItemKind, Span, Stmt, StmtKind, TypeExpr,
    UpdateOp,
};
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
    let ExprKind::Call { callee, args, .. } = &expr.kind else {
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
fn parses_while_and_do_while() {
    let output = ok(
        "function f(n: number) { while (n > 0) { n = n - 1; } do { n = n + 1; } while (n < 2); }",
    );
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    assert_eq!(decl.body.statements.len(), 2);
    assert!(matches!(decl.body.statements[0].kind, StmtKind::While(_)));
    assert!(matches!(decl.body.statements[1].kind, StmtKind::DoWhile(_)));
}

#[test]
fn parses_c_style_for_and_for_of() {
    let output = ok(
        "for (let i = 0; i < 3; i = i + 1) { console.log(i); } for (let x of xs) { console.log(x); }",
    );
    assert_eq!(output.program.items.len(), 2);
    let ItemKind::Stmt(Stmt {
        kind: StmtKind::For(stmt),
        ..
    }) = &output.program.items[0].kind
    else {
        panic!("expected a for loop");
    };
    assert!(stmt.init.is_some());
    assert!(stmt.condition.is_some());
    assert!(stmt.update.is_some());
    let ItemKind::Stmt(Stmt {
        kind: StmtKind::ForOf(stmt),
        ..
    }) = &output.program.items[1].kind
    else {
        panic!("expected a for-of loop");
    };
    assert_eq!(stmt.binding.name, "x");
}

#[test]
fn parses_break_and_continue() {
    let output = ok("function f() { while (true) { break; continue; } }");
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    let StmtKind::While(while_stmt) = &decl.body.statements[0].kind else {
        panic!("expected a while loop");
    };
    assert!(matches!(
        while_stmt.body.statements[0].kind,
        StmtKind::Break
    ));
    assert!(matches!(
        while_stmt.body.statements[1].kind,
        StmtKind::Continue
    ));
}

#[test]
fn parses_optional_member_and_index() {
    let output = ok("let v = a?.b; let w = a?.[0];");
    let ItemKind::Variable(first) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &first.init.as_ref().unwrap().kind,
        ExprKind::Member { optional: true, .. }
    ));
    let ItemKind::Variable(second) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &second.init.as_ref().unwrap().kind,
        ExprKind::Index { optional: true, .. }
    ));
}

#[test]
fn parses_arrow_function_forms() {
    let output = ok(
        "let add = (a: number, b: number): number => a + b; let sq = x => x * x; let f = () => { return 1; };",
    );
    let ItemKind::Variable(add) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    let ExprKind::Arrow {
        params, return_ty, ..
    } = &add.init.as_ref().unwrap().kind
    else {
        panic!("expected an arrow");
    };
    assert_eq!(params.len(), 2);
    assert!(return_ty.is_some());
    let ItemKind::Variable(sq) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &sq.init.as_ref().unwrap().kind,
        ExprKind::Arrow { params, .. } if params.len() == 1
    ));
    let ItemKind::Variable(f) = &output.program.items[2].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &f.init.as_ref().unwrap().kind,
        ExprKind::Arrow { params, .. } if params.is_empty()
    ));
}

#[test]
fn parses_compound_assignment_and_updates() {
    let output = ok("let x = 1; x += 2; x++; --x;");
    let ItemKind::Expr(add) = &output.program.items[1].kind else {
        panic!("expected an expression item");
    };
    assert!(matches!(
        add.kind,
        ExprKind::CompoundAssign {
            op: AssignOp::Add,
            ..
        }
    ));
    let ItemKind::Expr(post) = &output.program.items[2].kind else {
        panic!("expected an expression item");
    };
    assert!(matches!(
        post.kind,
        ExprKind::Update {
            op: UpdateOp::Inc,
            prefix: false,
            ..
        }
    ));
    let ItemKind::Expr(pre) = &output.program.items[3].kind else {
        panic!("expected an expression item");
    };
    assert!(matches!(
        pre.kind,
        ExprKind::Update {
            op: UpdateOp::Dec,
            prefix: true,
            ..
        }
    ));
}

#[test]
fn parses_ternary_and_right_associative_power() {
    let output = ok("let y = 1 > 0 ? 2 ** 3 ** 2 : 0;");
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    let ExprKind::Ternary {
        then_expr,
        else_expr,
        ..
    } = &decl.init.as_ref().unwrap().kind
    else {
        panic!("expected a ternary");
    };
    assert!(matches!(else_expr.kind, ExprKind::Number(0.0)));
    let ExprKind::Binary { op, right, .. } = &then_expr.kind else {
        panic!("expected a power expression");
    };
    assert_eq!(*op, BinaryOp::Pow);
    assert!(matches!(
        right.kind,
        ExprKind::Binary {
            op: BinaryOp::Pow,
            ..
        }
    ));
}

#[test]
fn decodes_based_literals_and_unicode_escapes() {
    let output = ok("let n = 0x10 + 0b11 + 0o7; let s = \"\\x41\\u{42}\\u0043\";");
    let ItemKind::Variable(numbers) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert!(format!("{:?}", numbers.init).contains("16.0"));
    let ItemKind::Variable(text) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &text.init.as_ref().unwrap().kind,
        ExprKind::String(value) if value == "ABC"
    ));
}

#[test]
fn parses_assignment_form_for_of() {
    let output = ok("for (x of xs) { console.log(x); }");
    let ItemKind::Stmt(Stmt {
        kind: StmtKind::ForOf(stmt),
        ..
    }) = &output.program.items[0].kind
    else {
        panic!("expected a for-of loop");
    };
    assert!(!stmt.decl);
    assert_eq!(stmt.binding.name, "x");
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

#[test]
fn parses_bigint_literals_in_multiple_radices() {
    let output = ok("let a = 123n; let b = 0xffn; let c = 1_000n;");
    let values: Vec<i64> = output
        .program
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::Variable(decl) => match &decl.init.as_ref().unwrap().kind {
                ExprKind::BigInt(value) => Some(*value),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(values, vec![123, 255, 1000]);
}

#[test]
fn diagnoses_out_of_range_bigint_literals() {
    let output = parse("let huge = 999999999999999999999999n;");
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("64-bit range"))
    );
}

#[test]
fn parses_optional_calls() {
    let output = ok("f?.(); f?.(1, \"two\");");
    for item in &output.program.items {
        let ItemKind::Expr(expr) = &item.kind else {
            panic!("expected an expression item");
        };
        assert!(matches!(&expr.kind, ExprKind::Call { optional: true, .. }));
    }
    let ItemKind::Expr(second) = &output.program.items[1].kind else {
        panic!("expected an expression item");
    };
    let ExprKind::Call { args, .. } = &second.kind else {
        panic!("expected a call");
    };
    assert_eq!(args.len(), 2);
}

#[test]
fn parses_imports_and_exports() {
    let output = ok(
        "import { a, b } from \"./m.ts\"; export function f(): number { return 1; } export const n = 2;",
    );
    let ItemKind::Import { names, source, .. } = &output.program.items[0].kind else {
        panic!("expected an import");
    };
    assert_eq!(names.len(), 2);
    assert_eq!(source, "./m.ts");
    assert!(
        matches!(&output.program.items[1].kind, ItemKind::Export(inner) if matches!(inner.kind, ItemKind::Function(_)))
    );
    assert!(
        matches!(&output.program.items[2].kind, ItemKind::Export(inner) if matches!(inner.kind, ItemKind::Variable(_)))
    );
}

#[test]
fn parses_union_intersection_and_optional_property_types() {
    let output = ok(
        "interface P { name: string; nickname?: string; } let u: string | null; let i: P & { age: number };",
    );
    let ItemKind::Interface(interface) = &output.program.items[0].kind else {
        panic!("expected an interface");
    };
    assert!(interface.fields[1].optional);
    let ItemKind::Variable(union) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    assert!(
        matches!(&union.ty.as_ref().unwrap().expr, TypeExpr::Union(members) if members.len() == 2)
    );
    let ItemKind::Variable(intersection) = &output.program.items[2].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &intersection.ty.as_ref().unwrap().expr,
        TypeExpr::Intersection(members) if members.len() == 2
    ));
}

#[test]
fn union_binds_looser_than_intersection() {
    let output = ok("let x: A & B | C;");
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    let TypeExpr::Union(members) = &decl.ty.as_ref().unwrap().expr else {
        panic!("expected a union at the top level");
    };
    assert!(matches!(&members[0], TypeExpr::Intersection(_)));
}

#[test]
fn parses_class_declarations_with_members() {
    let output = ok(
        "class Point extends Shape { x: number; constructor(x: number) { this.x = x; } sum(): number { return this.x; } }",
    );
    let ItemKind::Class(class) = &output.program.items[0].kind else {
        panic!("expected a class");
    };
    assert_eq!(class.name.name, "Point");
    assert_eq!(class.base.as_ref().unwrap().name, "Shape");
    assert_eq!(class.members.len(), 3);
    assert!(matches!(
        class.members[0],
        nava_ast::ClassMember::Field { .. }
    ));
    assert!(matches!(
        class.members[1],
        nava_ast::ClassMember::Constructor { .. }
    ));
    assert!(matches!(
        class.members[2],
        nava_ast::ClassMember::Method { .. }
    ));
}

#[test]
fn parses_new_this_and_super_expressions() {
    let output = ok("let p = new Point(1);");
    let ItemKind::Variable(decl) = &output.program.items[0].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &decl.init.as_ref().unwrap().kind,
        ExprKind::New { class, args } if class.name == "Point" && args.len() == 1
    ));
    let output = ok("function f(): number { let t = this; let s = super; return 1; }");
    let text = format!("{:?}", output.program);
    assert!(text.contains("This"));
    assert!(text.contains("Super"));
}

#[test]
fn parses_generic_function_declarations() {
    let output = ok("function identity<T>(value: T): T { return value; }");
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    assert_eq!(decl.type_params.len(), 1);
    assert_eq!(decl.type_params[0].name, "T");
}

#[test]
fn parses_multiple_type_parameters() {
    let output = ok("function pair<A, B>(a: A, b: B): A { return a; }");
    let ItemKind::Function(decl) = &output.program.items[0].kind else {
        panic!("expected a function");
    };
    let names: Vec<&str> = decl
        .type_params
        .iter()
        .map(|param| param.name.as_str())
        .collect();
    assert_eq!(names, vec!["A", "B"]);
}

#[test]
fn parses_generic_type_references_including_nested_forms() {
    let output =
        ok("interface Box<T> { value: T; } let b: Box<string>; let nested: Box<Box<number>>;");
    let ItemKind::Interface(interface) = &output.program.items[0].kind else {
        panic!("expected an interface");
    };
    assert_eq!(interface.type_params.len(), 1);
    let ItemKind::Variable(outer) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &outer.ty.as_ref().unwrap().expr,
        TypeExpr::Generic { name, args } if name == "Box" && args.len() == 1
    ));
    let ItemKind::Variable(nested) = &output.program.items[2].kind else {
        panic!("expected a variable");
    };
    let TypeExpr::Generic { args, .. } = &nested.ty.as_ref().unwrap().expr else {
        panic!("expected a generic type");
    };
    assert!(matches!(&args[0], TypeExpr::Generic { name, .. } if name == "Box"));
}

#[test]
fn parses_explicit_call_type_arguments() {
    let output =
        ok("function id<T>(x: T): T { return x; } let n = id<number>(1); let plain = id(2);");
    let ItemKind::Variable(explicit) = &output.program.items[1].kind else {
        panic!("expected a variable");
    };
    let ExprKind::Call { type_args, .. } = &explicit.init.as_ref().unwrap().kind else {
        panic!("expected a call");
    };
    assert_eq!(type_args.len(), 1);
    let ItemKind::Variable(plain) = &output.program.items[2].kind else {
        panic!("expected a variable");
    };
    assert!(matches!(
        &plain.init.as_ref().unwrap().kind,
        ExprKind::Call { type_args, .. } if type_args.is_empty()
    ));
}

#[test]
fn keeps_direct_calls_distinct_from_optional_calls() {
    let output = ok("f(); f?.();");
    let ItemKind::Expr(direct) = &output.program.items[0].kind else {
        panic!("expected an expression item");
    };
    assert!(matches!(
        &direct.kind,
        ExprKind::Call {
            optional: false,
            ..
        }
    ));
}
