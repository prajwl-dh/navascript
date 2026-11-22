//! Direct call lowering into IR instructions. Generic callees are specialized
//! by `Lowerer::specialize_if_generic`.

use nava_ast::Expr;

use super::builder::FunctionBuilder;
use super::ir::{Instruction, Value};
use super::lower::{Lowerer, ir_type_from_symbol};

impl FunctionBuilder {
    /// Lower `f(args)` and `f?.(args)`. Both become a direct call: Nava has no
    /// first-class function values, so there is no callable value that could be
    /// null and the optional branch is unobservable until that changes.
    pub(super) fn lower_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        _optional: bool,
        span: nava_ast::Span,
        lowerer: &mut Lowerer,
    ) -> Value {
        if let Some(mangled) = lowerer.typed.method_calls.get(&span).cloned() {
            let object = match &callee.kind {
                nava_ast::ExprKind::Member { object, .. } => self.lower_expr(object, lowerer),
                _ => {
                    lowerer.error("method call requires a receiver", callee.span);
                    Value::Null
                }
            };
            let mut call_args = vec![object];
            for arg in args {
                call_args.push(self.lower_expr(arg, lowerer));
            }
            let result_ty = lowerer
                .typed
                .types
                .get(&span)
                .map(|ty| ir_type_from_symbol(ty, &lowerer.current_subst))
                .unwrap_or(super::ir::IrType::Opaque);
            let result = if result_ty == super::ir::IrType::Void {
                None
            } else {
                Some(self.fresh_value())
            };
            self.emit(Instruction::Call {
                result,
                result_ty,
                callee: mangled,
                args: call_args,
            });
            return result.map_or(Value::Null, Value::Local);
        }
        let callee_name = match &callee.kind {
            nava_ast::ExprKind::Ident(name) => name.clone(),
            nava_ast::ExprKind::Member {
                object, property, ..
            } => match &object.kind {
                nava_ast::ExprKind::Ident(object) => format!("{object}.{property}"),
                _ => {
                    lowerer.error("unsupported call target in IR", callee.span);
                    "unknown".to_owned()
                }
            },
            _ => {
                lowerer.error("unsupported call target in IR", callee.span);
                "unknown".to_owned()
            }
        };
        let args: Vec<Value> = args
            .iter()
            .map(|arg| self.lower_expr(arg, lowerer))
            .collect();
        if (callee_name.contains('.') || callee_name == "unknown")
            && !Lowerer::is_known_builtin(&callee_name)
        {
            lowerer.error(
                format!("unsupported call target '{callee_name}' in IR"),
                callee.span,
            );
        }

        let (emitted_name, result_ty) = lowerer.specialize_if_generic(&callee_name, span);
        let result = match result_ty {
            super::ir::IrType::Void => None,
            _ => Some(self.fresh_value()),
        };
        self.emit(Instruction::Call {
            result,
            result_ty,
            callee: emitted_name,
            args,
        });
        result.map_or(Value::Null, Value::Local)
    }
}
