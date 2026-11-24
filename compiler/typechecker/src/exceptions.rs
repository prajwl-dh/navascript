//! Exception checking: throw targets, catch bindings and protected regions
//! (ADR-0028). Kept private behind the `check` seam.

use navascript_ast::{Block, Expr, Span, TryStmt};
use navascript_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::Subst;

/// Exception state saved when entering a new function (a named function or an
/// arrow). A handler in an enclosing function cannot catch a throw here.
pub(super) struct ExceptionContext {
    catches: Vec<Type>,
    protected: u32,
    forbidden: u32,
}

impl Checker<'_> {
    pub(super) fn suspend_exception_context(&mut self) -> ExceptionContext {
        ExceptionContext {
            catches: std::mem::take(&mut self.catch_types),
            protected: std::mem::take(&mut self.protected_depth),
            forbidden: std::mem::take(&mut self.throw_forbidden),
        }
    }

    pub(super) fn restore_exception_context(&mut self, context: ExceptionContext) {
        self.catch_types = context.catches;
        self.protected_depth = context.protected;
        self.throw_forbidden = context.forbidden;
    }

    /// `return`, `break` and `continue` cannot cross a protected region in the
    /// current lowering, so they are diagnosed rather than silently skipped.
    pub(super) fn reject_abrupt(&mut self, word: &str, span: Span) {
        if self.protected_depth > 0 {
            self.error(
                format!("{word} inside try/catch/finally is not supported"),
                span,
            );
        }
    }

    pub(super) fn check_throw(&mut self, expr: &Expr, span: Span) {
        let actual = self.infer_expr(expr);
        if self.throw_forbidden > 0 {
            self.error("throw inside catch/finally is not supported", span);
            return;
        }
        if let Some(expected) = self.catch_types.last().cloned() {
            self.check_assignment(&expected, &actual, span);
        } else {
            self.error(
                "throw requires an enclosing try in the same function (cross-function unwinding is not supported)",
                span,
            );
        }
    }

    /// Check a `try`: the catch type constrains every throw in the try body,
    /// and is recorded on the catch binding's span for IR lowering (so type
    /// aliases resolve to a concrete IR type).
    pub(super) fn check_try(&mut self, stmt: &TryStmt) {
        let catch_ty = match &stmt.catch_param.ty {
            Some(annotation) => self.resolve(&annotation.expr, &Subst::new(), annotation.span),
            None => Type::Error,
        };
        self.types
            .insert(stmt.catch_param.name.span, catch_ty.clone());

        self.catch_types.push(catch_ty.clone());
        self.check_protected_block(&stmt.body, false);
        self.catch_types.pop();

        self.symbols.enter_scope();
        self.symbols.insert(Symbol {
            name: stmt.catch_param.name.name.clone(),
            kind: SymbolKind::Variable,
            ty: catch_ty,
            span: stmt.catch_param.name.span,
        });
        self.check_protected_block(&stmt.catch_body, true);
        self.symbols.leave_scope();

        if let Some(finally) = &stmt.finally_body {
            self.check_protected_block(finally, true);
        }
    }

    fn check_protected_block(&mut self, block: &Block, forbid_throw: bool) {
        if forbid_throw {
            self.throw_forbidden += 1;
        }
        self.protected_depth += 1;
        self.check_block(block, None);
        self.protected_depth -= 1;
        if forbid_throw {
            self.throw_forbidden -= 1;
        }
    }
}
