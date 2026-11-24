//! Name resolution and shared capture cells (ADR-0027).
//!
//! A captured binding is a one-slot runtime object shared by the enclosing
//! frame and every closure that captures it. Reads and writes go through the
//! cell, so closures observe later assignments in the enclosing scope.

use std::collections::HashMap;

use navascript_ast::Ident;

use super::builder::FunctionBuilder;
use super::ir::{Instruction, IrType, SlotId, Value};

/// A name resolved inside a function: a local stack slot (possibly boxed into
/// a shared cell), or a field of the closure environment passed as parameter 0.
#[derive(Debug, Clone)]
pub(super) enum Binding {
    Slot(SlotId, IrType),
    Cell {
        slot: SlotId,
        cell: IrType,
        value: IrType,
    },
    Capture(u32, IrType),
    CaptureCell {
        index: u32,
        cell: IrType,
        value: IrType,
    },
}

impl Binding {
    pub(super) fn value_type(&self) -> IrType {
        match self {
            Binding::Slot(_, ty) | Binding::Capture(_, ty) => ty.clone(),
            Binding::Cell { value, .. } | Binding::CaptureCell { value, .. } => value.clone(),
        }
    }
}

/// How an environment field stores a captured name.
#[derive(Debug, Clone)]
pub(super) enum Captured {
    /// A value copied into the environment (used for `this`, never reassigned).
    Value(IrType),
    /// A shared cell pointer: writes are visible on both sides of the closure.
    Cell { cell: IrType, value: IrType },
}

impl Captured {
    pub(super) fn value_type(&self) -> IrType {
        match self {
            Captured::Value(ty) => ty.clone(),
            Captured::Cell { value, .. } => value.clone(),
        }
    }
}

/// One-slot runtime object used as a shared captured binding.
pub(super) fn cell_type(value: &IrType) -> IrType {
    IrType::Object(vec![("__value".to_owned(), value.clone())])
}

impl FunctionBuilder {
    /// Declare a binding as a shared cell so closures see later writes.
    pub(super) fn bind_cell(&mut self, name: &Ident, value: Value, value_ty: IrType) {
        let cell_ty = cell_type(&value_ty);
        let cell = self.fresh_value();
        self.emit(Instruction::ObjectNew {
            result: cell,
            fields: vec![("__value".to_owned(), value_ty.clone())],
        });
        self.emit(Instruction::ObjectSet {
            object: Value::Local(cell),
            field_index: 0,
            value,
            field_ty: value_ty.clone(),
        });
        let slot = self.alloc(cell_ty.clone());
        self.store(slot, cell_ty.clone(), Value::Local(cell));
        self.boxed
            .insert(name.name.clone(), (slot, cell_ty, value_ty));
        self.declare(name, slot);
    }

    /// Resolve a name to a local binding first, then to a captured field. Local
    /// declarations shadow captures, preserving lexical scoping.
    pub(super) fn lookup_binding(&self, name: &str) -> Option<Binding> {
        if let Some(&(slot, ref cell, ref value)) = self.boxed.get(name) {
            return Some(Binding::Cell {
                slot,
                cell: cell.clone(),
                value: value.clone(),
            });
        }
        self.lookup(name)
            .map(|(slot, ty)| Binding::Slot(slot, ty))
            .or_else(|| {
                self.captures
                    .get(name)
                    .map(|(index, captured)| match captured {
                        Captured::Value(ty) => Binding::Capture(*index, ty.clone()),
                        Captured::Cell { cell, value } => Binding::CaptureCell {
                            index: *index,
                            cell: cell.clone(),
                            value: value.clone(),
                        },
                    })
            })
    }

    /// The value type of every name visible to a nested arrow.
    pub(super) fn available_bindings(&self) -> HashMap<String, IrType> {
        let mut out: HashMap<String, IrType> = HashMap::new();
        for scope in &self.scopes {
            for (name, slot) in scope {
                if let Some(ty) = self.slot_type(*slot) {
                    let value_ty = self
                        .boxed
                        .get(name)
                        .map(|(_, _, value)| value.clone())
                        .unwrap_or(ty);
                    out.insert(name.clone(), value_ty);
                }
            }
        }
        for (name, (_, captured)) in &self.captures {
            out.insert(name.clone(), captured.value_type());
        }
        if let Some((_, ty)) = &self.this_capture {
            out.insert("this".to_owned(), ty.clone());
        } else if let Some(ty) = self.this_ty.clone() {
            out.insert("this".to_owned(), ty);
        }
        out
    }

    /// Read a binding from the current frame or the closure environment.
    pub(super) fn load_binding(&mut self, binding: &Binding) -> Value {
        match binding {
            Binding::Slot(slot, ty) => self.load(*slot, ty.clone()),
            Binding::Cell { slot, cell, value } => {
                let pointer = self.load(*slot, cell.clone());
                self.load_cell(pointer, value)
            }
            Binding::Capture(index, ty) => self.env_get(*index, ty),
            Binding::CaptureCell { index, cell, value } => {
                let pointer = self.env_get(*index, cell);
                self.load_cell(pointer, value)
            }
        }
    }

    /// Write a binding in the current frame or the closure environment.
    pub(super) fn store_binding(&mut self, binding: &Binding, value: Value) {
        match binding {
            Binding::Slot(slot, ty) => self.store(*slot, ty.clone(), value),
            Binding::Cell { slot, cell, .. } => {
                let pointer = self.load(*slot, cell.clone());
                self.store_cell(pointer, value, binding.value_type());
            }
            Binding::Capture(index, ty) => self.emit(Instruction::ObjectSet {
                object: Value::Param(0),
                field_index: *index,
                value,
                field_ty: ty.clone(),
            }),
            Binding::CaptureCell {
                index,
                cell,
                value: value_ty,
            } => {
                let pointer = self.env_get(*index, cell);
                self.store_cell(pointer, value, value_ty.clone());
            }
        }
    }

    /// The storage a closure should capture: a shared cell pointer for boxed
    /// bindings, the value itself for everything else.
    pub(super) fn capture_storage(&mut self, binding: &Binding) -> (IrType, Value) {
        match binding {
            Binding::Cell { slot, cell, .. } => (cell.clone(), self.load(*slot, cell.clone())),
            Binding::CaptureCell { index, cell, .. } => (cell.clone(), self.env_get(*index, cell)),
            other => {
                let ty = other.value_type();
                (ty.clone(), self.load_binding(other))
            }
        }
    }

    fn env_get(&mut self, index: u32, ty: &IrType) -> Value {
        let result = self.fresh_value();
        self.emit(Instruction::ObjectGet {
            result,
            object: Value::Param(0),
            field_index: index,
            field_ty: ty.clone(),
        });
        Value::Local(result)
    }

    fn load_cell(&mut self, pointer: Value, value: &IrType) -> Value {
        let result = self.fresh_value();
        self.emit(Instruction::ObjectGet {
            result,
            object: pointer,
            field_index: 0,
            field_ty: value.clone(),
        });
        Value::Local(result)
    }

    fn store_cell(&mut self, pointer: Value, value: Value, ty: IrType) {
        self.emit(Instruction::ObjectSet {
            object: pointer,
            field_index: 0,
            value,
            field_ty: ty,
        });
    }
}
