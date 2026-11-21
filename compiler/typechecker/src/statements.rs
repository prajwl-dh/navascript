//! Declaration checking, blocks and control-flow statements.

use nava_ast::{Block, FunctionDecl, IfStmt, Stmt, StmtKind, VariableDecl};
use nava_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::{is_assignable, resolve_type};

impl<'a> Checker<'a> {
    pub(super) fn check_variable(&mut self, decl: &VariableDecl, top_level: bool) {
        let declared = decl
            .ty
            .as_ref()
            .map(|annotation| resolve_type(&annotation.expr, &self.aliases));
        let inferred = decl.init.as_ref().map(|expr| self.infer_expr(expr));

        if let (Some(expected), Some(actual)) = (&declared, &inferred) {
            if !is_assignable(expected, actual, &self.aliases) {
                self.error(
                    format!(
                        "cannot assign '{}' to '{}'",
                        actual.display(),
                        expected.display()
                    ),
                    decl.init.as_ref().expect("actual exists").span,
                );
            }
        }

        let final_ty = match (declared, inferred) {
            (Some(expected), _) => expected,
            (None, Some(actual)) => actual,
            (None, None) => Type::Unknown,
        };
        // Top-level declarations were inserted in the collection pass, where
        // duplicates were already reported. Locals are inserted here, so a
        // duplicate in the same scope is reported now.
        let previous = self.symbols.insert(Symbol {
            name: decl.name.name.clone(),
            kind: SymbolKind::Variable,
            ty: final_ty,
            span: decl.name.span,
        });
        if !top_level && previous.is_some() {
            self.error(
                format!("duplicate declaration '{}'", decl.name.name),
                decl.name.span,
            );
        }
    }

    pub(super) fn check_function(&mut self, decl: &FunctionDecl) {
        self.symbols.enter_scope();
        for param in &decl.params {
            let ty = param
                .ty
                .as_ref()
                .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
                .unwrap_or(Type::Unknown);
            self.symbols.insert(Symbol {
                name: param.name.name.clone(),
                kind: SymbolKind::Parameter,
                ty,
                span: param.name.span,
            });
        }
        let expected_ret = decl
            .return_ty
            .as_ref()
            .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
            .unwrap_or_else(|| self.infer_return_type(&decl.body));

        self.check_block(&decl.body, Some(&expected_ret));
        self.symbols.leave_scope();

        // Refresh the *global* signature so later functions and IR lowering
        // see the inferred return type, not the placeholder from pass one.
        // This must happen after leaving the parameter scope.
        if let Some(Type::Function { params, .. }) = self
            .symbols
            .lookup(&decl.name.name)
            .map(|symbol| symbol.ty.clone())
        {
            self.symbols.insert(Symbol {
                name: decl.name.name.clone(),
                kind: SymbolKind::Function,
                ty: Type::Function {
                    params,
                    ret: Box::new(expected_ret),
                },
                span: decl.name.span,
            });
        }
    }

    pub(super) fn check_block(&mut self, block: &Block, expected_ret: Option<&Type>) {
        self.symbols.enter_scope();
        for statement in &block.statements {
            self.check_stmt(statement, expected_ret);
        }
        self.symbols.leave_scope();
    }

    pub(super) fn check_stmt(&mut self, statement: &Stmt, expected_ret: Option<&Type>) {
        match &statement.kind {
            StmtKind::Return(value) => {
                let actual = value
                    .as_ref()
                    .map(|expr| self.infer_expr(expr))
                    .unwrap_or(Type::Void);
                if let Some(expected) = expected_ret {
                    if !is_assignable(expected, &actual, &self.aliases) {
                        self.error(
                            format!(
                                "return type '{}' is not assignable to '{}'",
                                actual.display(),
                                expected.display()
                            ),
                            statement.span,
                        );
                    }
                }
            }
            StmtKind::Variable(decl) => self.check_variable(decl, false),
            StmtKind::If(if_stmt) => self.check_if(if_stmt, expected_ret),
            StmtKind::Block(block) => self.check_block(block, expected_ret),
            StmtKind::Expr(expr) => {
                self.infer_expr(expr);
            }
            StmtKind::Empty => {}
        }
    }

    pub(super) fn check_if(&mut self, if_stmt: &IfStmt, expected_ret: Option<&Type>) {
        self.infer_expr(&if_stmt.condition);
        self.check_block(&if_stmt.then_branch, expected_ret);
        match &if_stmt.else_branch {
            Some(nava_ast::ElseBranch::If(nested)) => self.check_if(nested, expected_ret),
            Some(nava_ast::ElseBranch::Block(block)) => self.check_block(block, expected_ret),
            None => {}
        }
    }
}
