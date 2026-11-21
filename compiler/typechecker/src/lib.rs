//! Nava type checker — the Phase 2 external seam.
//!
//! `check(&Program, &str) -> CheckOutput` hides the symbol table, scope
//! handling, type inference and assignability rules. It depends only on the
//! AST, the shared diagnostics leaf and the symbol/type leaf; LLVM is not
//! involved.

mod checker;
mod types;

pub use checker::{CheckOutput, TypedProgram, check};
pub use types::{AliasTable, is_assignable, resolve_type};

#[cfg(test)]
mod tests {
    use nava_ast::{ItemKind, Program};
    use nava_parser::parse;
    use nava_symbol::Type;

    use super::*;

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
    fn rejects_comparisons_of_non_orderable_types() {
        let (output, _) = check_source("let x = true < false;");
        assert!(
            messages(&output)
                .iter()
                .any(|message| message.contains("comparison requires two numbers or two strings"))
        );
    }
}
