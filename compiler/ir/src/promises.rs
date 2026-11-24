//! Promise value layout and helpers (ADR-0029).
//!
//! A promise is an ordinary runtime object with one `__value` slot. `await`
//! reads that slot synchronously — there is no suspension or event loop. The
//! layout lives here so lowering, return wrapping and `await` cannot disagree.

use navascript_ast::Expr;
use navascript_symbol::Type;

use super::builder::FunctionBuilder;
use super::ir::{Instruction, IrType, Value};
use super::lower::{Lowerer, ir_type_from_symbol};

pub(super) const VALUE_FIELD: &str = "__value";

/// IR type of a promise carrying `inner`. A `Promise<void>` has no value, so
/// it maps to `void` rather than an object with an unusable `void` slot.
pub(super) fn promise_type(inner: IrType) -> IrType {
    if inner == IrType::Void {
        IrType::Void
    } else {
        IrType::Object(vec![(VALUE_FIELD.to_owned(), inner)])
    }
}

impl FunctionBuilder {
    /// Wrap a returned value in a promise object.
    pub(super) fn wrap_in_promise(&mut self, value: Value, inner_ty: IrType) -> Value {
        let promise = self.fresh_value();
        self.emit(Instruction::ObjectNew {
            result: promise,
            fields: vec![(VALUE_FIELD.to_owned(), inner_ty.clone())],
        });
        self.emit(Instruction::ObjectSet {
            object: Value::Local(promise),
            field_index: 0,
            value,
            field_ty: inner_ty,
        });
        Value::Local(promise)
    }

    /// Unwrap a promise for `await`. A void promise carries no value.
    pub(super) fn unwrap_promise(&mut self, promise: Value, inner_ty: IrType) -> Value {
        if inner_ty == IrType::Void {
            return Value::Null;
        }
        let result = self.fresh_value();
        self.emit(Instruction::ObjectGet {
            result,
            object: promise,
            field_index: 0,
            field_ty: inner_ty,
        });
        Value::Local(result)
    }
}

impl Lowerer<'_> {
    /// The value wrapped by a `Promise<T>` expression, used by `await`.
    pub(super) fn promise_inner_type(&self, expr: &Expr) -> IrType {
        self.typed
            .types
            .get(&expr.span)
            .and_then(|ty| match ty {
                Type::Promise(inner) => Some(ir_type_from_symbol(inner, &self.current_subst)),
                _ => None,
            })
            .unwrap_or(IrType::Opaque)
    }
}
