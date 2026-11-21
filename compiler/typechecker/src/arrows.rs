//! Arrow function type inference.

use nava_ast::{ArrowBody, Param, TypeAnnotation};
use nava_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::resolve_type;

impl Checker<'_> {
    pub(super) fn infer_arrow(
        &mut self,
        params: &[Param],
        return_ty: &Option<TypeAnnotation>,
        body: &ArrowBody,
    ) -> Type {
        self.symbols.enter_scope();
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
            let ty = resolve_type(&annotation.expr, &self.aliases);
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
        self.symbols.leave_scope();
        let ret = return_ty
            .as_ref()
            .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
            .unwrap_or(inferred);
        Type::Function {
            params: param_types,
            ret: Box::new(ret),
        }
    }
}
