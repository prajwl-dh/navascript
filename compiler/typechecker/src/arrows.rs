//! Arrow function type inference.

use navascript_ast::{ArrowBody, Param, TypeAnnotation};
use navascript_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::Subst;

impl Checker<'_> {
    pub(super) fn infer_arrow(
        &mut self,
        params: &[Param],
        return_ty: &Option<TypeAnnotation>,
        body: &ArrowBody,
    ) -> Type {
        self.symbols.enter_scope();
        // An arrow is a separate function: a `throw` inside it cannot use a
        // handler from the enclosing function (ADR-0028).
        let outer_exceptions = self.suspend_exception_context();
        let outer_async = self.current_function_async.replace(false);
        let mut param_types = Vec::new();
        for param in params {
            let Some(annotation) = &param.ty else {
                self.error(
                    "arrow function parameters require a type annotation",
                    param.span,
                );
                param_types.push(Type::Error);
                continue;
            };
            let ty = self.resolve(&annotation.expr, &Subst::new(), annotation.span);
            self.symbols.insert(Symbol {
                name: param.name.name.clone(),
                kind: SymbolKind::Parameter,
                ty: ty.clone(),
                span: param.name.span,
            });
            param_types.push(ty);
        }
        let inferred = match body {
            ArrowBody::Expr(inner) => self.infer_expr(inner),
            ArrowBody::Block(block) => {
                let inferred = self.infer_return_type(block);
                self.check_block(block, Some(&inferred));
                inferred
            }
        };
        self.restore_exception_context(outer_exceptions);
        self.current_function_async = outer_async;
        self.symbols.leave_scope();
        let ret = return_ty
            .as_ref()
            .map(|annotation| self.resolve(&annotation.expr, &Subst::new(), annotation.span))
            .unwrap_or(inferred);
        Type::Function {
            params: param_types,
            ret: Box::new(ret),
        }
    }
}
