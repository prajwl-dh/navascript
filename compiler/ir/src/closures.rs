//! First-class function values and closures (ADR-0027).
//!
//! A function value is a runtime object: slot 0 holds the code pointer and the
//! remaining slots hold captured bindings in sorted name order. Captured
//! bindings are shared one-slot cells, so writes from a closure and from its
//! enclosing frame are mutually visible. Closure functions take the object as
//! a hidden first parameter; calls through a value become `CallIndirect`.

use navascript_ast::{ArrowBody, Expr, ExprKind, Param, Span};
use navascript_symbol::Type;

use super::bindings::{Binding, Captured};
use super::builder::FunctionBuilder;
use super::closure_names::{captures_in_body, free_names};
use super::ir::{Instruction, IrParam, IrType, Terminator, Value};
use super::lower::{Lowerer, ir_type, ir_type_from_symbol};

/// The LLVM-level type of a captured environment: an empty object compiles to
/// `ptr`, which is all the hidden parameter ever needs to be.
fn env_type() -> IrType {
    IrType::Object(Vec::new())
}

/// One captured field of a closure, with its storage and value types.
#[derive(Debug, Clone)]
pub(super) struct CaptureSpec {
    pub(super) name: String,
    pub(super) storage_ty: IrType,
    pub(super) storage: Value,
    pub(super) value_ty: IrType,
    pub(super) captured: Captured,
}

impl FunctionBuilder {
    /// Lower an arrow expression to a closure object allocated in the current
    /// function. The generated closure function is appended to the module.
    pub(super) fn lower_arrow(
        &mut self,
        expr: &Expr,
        params: &[Param],
        body: &ArrowBody,
        lowerer: &mut Lowerer,
    ) -> Value {
        let fn_ty = match lowerer.expr_ty(expr) {
            IrType::Function { params, ret } => IrType::Function { params, ret },
            _ => {
                lowerer.error(
                    "cannot determine the type of this arrow function",
                    expr.span,
                );
                IrType::Function {
                    params: params.iter().map(|param| ir_type(&param.ty)).collect(),
                    ret: Box::new(IrType::Opaque),
                }
            }
        };
        let visible = self.available_bindings();
        let mut captures: Vec<CaptureSpec> = Vec::new();
        for name in free_names(params, body) {
            let Some(value_ty) = visible.get(&name) else {
                continue;
            };
            let (storage_ty, storage, captured) = if name == "this" {
                let expr = Expr {
                    kind: ExprKind::This,
                    span: expr.span,
                };
                let value = self.lower_self(&expr, lowerer);
                (value_ty.clone(), value, Captured::Value(value_ty.clone()))
            } else {
                let Some(binding) = self.lookup_binding(&name) else {
                    continue;
                };
                let (storage_ty, storage) = self.capture_storage(&binding);
                let captured = match binding {
                    Binding::Cell { .. } | Binding::CaptureCell { .. } => Captured::Cell {
                        cell: storage_ty.clone(),
                        value: value_ty.clone(),
                    },
                    _ => Captured::Value(value_ty.clone()),
                };
                (storage_ty, storage, captured)
            };
            captures.push(CaptureSpec {
                name,
                storage_ty,
                storage,
                value_ty: value_ty.clone(),
                captured,
            });
        }
        captures.sort_by(|a, b| a.name.cmp(&b.name));
        let name = lowerer.fresh_closure_name("__navascript_arrow_");
        let function =
            build_closure_function(&name, params, body, &fn_ty, &captures, expr.span, lowerer);
        lowerer.module.functions.push(function);
        let code = Value::Function {
            name,
            ty: Box::new(fn_ty.clone()),
        };
        self.make_closure(code, fn_ty, &captures)
    }

    /// Wrap a named function in a zero-capture closure so it can be passed as a
    /// value. A thunk adapts the ordinary signature to the closure convention
    /// (environment pointer first), leaving direct calls untouched.
    pub(super) fn named_function_value(
        &mut self,
        name: &str,
        span: Span,
        lowerer: &mut Lowerer,
    ) -> Value {
        let fn_ty = match lowerer.typed.symbols.lookup(name) {
            Some(symbol) if matches!(symbol.ty, Type::Function { .. }) => {
                ir_type_from_symbol(&symbol.ty, &lowerer.current_subst)
            }
            _ => {
                lowerer.error(format!("'{name}' is not a callable value"), span);
                return Value::Null;
            }
        };
        let thunk = match lowerer.function_values.get(name).cloned() {
            Some(thunk) => thunk,
            None => {
                let thunk = lowerer
                    .fresh_closure_name(&format!("__navascript_fnvalue_{}_", llvm_safe_name(name)));
                lowerer
                    .module
                    .functions
                    .push(thunk_function(&thunk, name, &fn_ty));
                lowerer
                    .function_values
                    .insert(name.to_owned(), thunk.clone());
                thunk
            }
        };
        let code = Value::Function {
            name: thunk,
            ty: Box::new(fn_ty.clone()),
        };
        self.make_closure(code, fn_ty, &[])
    }

    /// Allocate a closure object and fill its code pointer and capture slots.
    fn make_closure(&mut self, code: Value, fn_ty: IrType, captures: &[CaptureSpec]) -> Value {
        let fields = captures
            .iter()
            .map(|capture| {
                (
                    capture.name.clone(),
                    capture.storage_ty.clone(),
                    capture.storage.clone(),
                )
            })
            .collect();
        let result = self.fresh_value();
        self.emit(Instruction::ClosureNew {
            result,
            code,
            fn_ty,
            captures: fields,
        });
        Value::Local(result)
    }
}

/// Build the closure function for an arrow. The environment is parameter 0.
fn build_closure_function(
    name: &str,
    params: &[Param],
    body: &ArrowBody,
    fn_ty: &IrType,
    captures: &[CaptureSpec],
    span: Span,
    lowerer: &mut Lowerer,
) -> super::ir::IrFunction {
    let ret = match fn_ty {
        IrType::Function { ret, .. } => (**ret).clone(),
        _ => IrType::Opaque,
    };
    let mut ir_params = vec![IrParam {
        name: "__env".to_owned(),
        ty: env_type(),
    }];
    for (index, param) in params.iter().enumerate() {
        ir_params.push(IrParam {
            name: param.name.name.clone(),
            ty: fn_param_ty(fn_ty, index, param),
        });
    }
    let mut builder = FunctionBuilder::new(name.to_owned(), ir_params, ret.clone());
    builder.set_captured_names(captures_in_body(body));
    for (index, param) in params.iter().enumerate() {
        let ty = fn_param_ty(fn_ty, index, param);
        if builder.captured_names.contains(&param.name.name) {
            builder.bind_cell(&param.name, Value::Param(index as u32 + 1), ty);
        } else {
            let slot = builder.alloc(ty.clone());
            builder.store(slot, ty, Value::Param(index as u32 + 1));
            builder.declare(&param.name, slot);
        }
    }
    for (index, capture) in captures.iter().enumerate() {
        let field = index as u32 + 1;
        builder
            .captures
            .insert(capture.name.clone(), (field, capture.captured.clone()));
        if capture.name == "this" {
            builder.this_capture = Some((field, capture.value_ty.clone()));
        }
    }
    let fallthrough = match body {
        ArrowBody::Block(block) => builder.lower_block(block, lowerer, builder.function.entry),
        ArrowBody::Expr(expr) => {
            let value = builder.lower_expr(expr, lowerer);
            let entry = builder.function.entry;
            builder.set_terminator(entry, Terminator::Return(Some(value)));
            None
        }
    };
    if let Some(block) = fallthrough {
        let value = if ret == IrType::Void {
            None
        } else {
            lowerer.error(
                format!("arrow function '{name}' may fall through without returning"),
                span,
            );
            Some(Value::Null)
        };
        builder.set_terminator(block, Terminator::Return(value));
    }
    builder.finish()
}

fn fn_param_ty(fn_ty: &IrType, index: usize, param: &Param) -> IrType {
    match fn_ty {
        IrType::Function { params, .. } => params
            .get(index)
            .cloned()
            .unwrap_or_else(|| ir_type(&param.ty)),
        _ => ir_type(&param.ty),
    }
}

/// Build the thunk body for a named function used as a value.
fn thunk_function(name: &str, target: &str, fn_ty: &IrType) -> super::ir::IrFunction {
    let ret = match fn_ty {
        IrType::Function { ret, .. } => (**ret).clone(),
        _ => IrType::Opaque,
    };
    let mut params = vec![IrParam {
        name: "__env".to_owned(),
        ty: env_type(),
    }];
    if let IrType::Function { params: user, .. } = fn_ty {
        for (index, ty) in user.iter().enumerate() {
            params.push(IrParam {
                name: format!("p{index}"),
                ty: ty.clone(),
            });
        }
    }
    let mut builder = FunctionBuilder::new(name.to_owned(), params, ret.clone());
    let args = (0..builder.function.params.len().saturating_sub(1))
        .map(|index| Value::Param(index as u32 + 1))
        .collect();
    let result = if ret == IrType::Void {
        None
    } else {
        Some(builder.fresh_value())
    };
    let entry = builder.function.entry;
    builder.emit(Instruction::Call {
        result,
        result_ty: ret,
        callee: target.to_owned(),
        args,
    });
    builder.set_terminator(entry, Terminator::Return(result.map(Value::Local)));
    builder.finish()
}

fn llvm_safe_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
