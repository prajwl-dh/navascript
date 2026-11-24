//! Call lowering: direct named calls, static method dispatch and calls through
//! first-class function values (ADR-0027).

use navascript_ast::{Expr, ExprKind};

use super::builder::FunctionBuilder;
use super::ir::{Instruction, IrType, Value};
use super::lower::Lowerer;

impl FunctionBuilder {
    /// Emit a call through a function value, passing the closure object as the
    /// hidden environment argument (ADR-0027).
    pub(super) fn emit_indirect_call(
        &mut self,
        callee: Value,
        fn_ty: IrType,
        result_ty: IrType,
        args: Vec<Value>,
        lowerer: &mut Lowerer,
        span: navascript_ast::Span,
    ) -> Value {
        if !matches!(fn_ty, IrType::Function { .. }) {
            lowerer.error(
                format!("cannot call a value of type '{}'", fn_ty.display()),
                span,
            );
            return Value::Null;
        }
        let result = if result_ty == IrType::Void {
            None
        } else {
            Some(self.fresh_value())
        };
        self.emit(Instruction::CallIndirect {
            result,
            result_ty,
            callee,
            args,
            fn_ty,
        });
        result.map_or(Value::Null, Value::Local)
    }

    /// Lower `f(args)`, `f?.(args)` and `(expr)(args)`.
    ///
    /// A named top-level function stays a direct call. A function-typed
    /// binding or an arbitrary callable expression becomes an indirect call
    /// through the closure object (the optional branch is still unobservable
    /// until nullable callables exist).
    pub(super) fn lower_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        _optional: bool,
        span: navascript_ast::Span,
        lowerer: &mut Lowerer,
    ) -> Value {
        if let Some(mangled) = lowerer.typed.method_calls.get(&span).cloned() {
            let object = match &callee.kind {
                ExprKind::Member { object, .. } => self.lower_expr(object, lowerer),
                _ => {
                    lowerer.error("method call requires a receiver", callee.span);
                    Value::Null
                }
            };
            let mut call_args = vec![object];
            for arg in args {
                call_args.push(self.lower_expr(arg, lowerer));
            }
            let result_ty = lowerer.call_result_type(span);
            let result = if result_ty == IrType::Void {
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

        // A local binding of function type: load the closure value and call it.
        if let ExprKind::Ident(name) = &callee.kind {
            if let Some(binding) = self.lookup_binding(name) {
                let ty = binding.value_type();
                if matches!(ty, IrType::Function { .. }) {
                    let callee_value = self.load_binding(&binding);
                    let call_args = args
                        .iter()
                        .map(|arg| self.lower_expr(arg, lowerer))
                        .collect();
                    return self.emit_indirect_call(
                        callee_value,
                        ty,
                        lowerer.call_result_type(span),
                        call_args,
                        lowerer,
                        span,
                    );
                }
            }
        }

        // Any other callee expression that the checker gave a function type
        // (an arrow literal, a returned closure, a call result, a callable
        // object field, ...) is called through its closure object. A member of
        // a local binding is such a value; `console.log` and module namespace
        // members are named functions and stay on the direct path below.
        let is_named_member = match &callee.kind {
            ExprKind::Member { object, .. } => match &object.kind {
                ExprKind::Ident(name) => self.lookup_binding(name).is_none(),
                _ => false,
            },
            _ => false,
        };
        if !matches!(callee.kind, ExprKind::Ident(_)) && !is_named_member {
            let callee_value = self.lower_expr(callee, lowerer);
            let fn_ty = lowerer.expr_ty(callee);
            let call_args = args
                .iter()
                .map(|arg| self.lower_expr(arg, lowerer))
                .collect();
            return self.emit_indirect_call(
                callee_value,
                fn_ty,
                lowerer.call_result_type(span),
                call_args,
                lowerer,
                span,
            );
        }

        // Ordinary named-call path, including generic specialization.
        let callee_name = match &callee.kind {
            ExprKind::Ident(name) => name.clone(),
            ExprKind::Member {
                object, property, ..
            } => match &object.kind {
                ExprKind::Ident(object) => format!("{object}.{property}"),
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
            IrType::Void => None,
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
