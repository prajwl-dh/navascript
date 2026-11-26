//! Operand and result accessors for IR instructions.
//!
//! These support the verifier and the optimizer; keeping them here leaves the
//! `ir.rs` data model within ADR-0016's size guidance without changing the
//! public interface.

use super::ir::{Instruction, SlotId, Terminator, Value, ValueId};

impl Instruction {
    /// Every value operand of this instruction, in source order.
    pub fn values(&self) -> Vec<&Value> {
        match self {
            Instruction::Store { value, .. }
            | Instruction::IsNull { value, .. }
            | Instruction::ToBool { value, .. } => vec![value],
            Instruction::Binary { lhs, rhs, .. }
            | Instruction::BigIntBinary { lhs, rhs, .. }
            | Instruction::StringConcat {
                left: lhs,
                right: rhs,
                ..
            }
            | Instruction::StringEquals {
                left: lhs,
                right: rhs,
                ..
            } => vec![lhs, rhs],
            Instruction::Unary { operand, .. }
            | Instruction::StringLength { value: operand, .. }
            | Instruction::NumberToString { value: operand, .. } => vec![operand],
            Instruction::StringCharAt { value, index, .. } => vec![value, index],
            Instruction::Call { args, .. } => args.iter().collect(),
            Instruction::CallIndirect { callee, args, .. } => {
                let mut values = vec![callee];
                values.extend(args.iter());
                values
            }
            Instruction::ArrayGet { array, index, .. } => vec![array, index],
            Instruction::ArraySet {
                array,
                index,
                value,
                ..
            } => vec![array, index, value],
            Instruction::ArrayLength { array, .. } => vec![array],
            Instruction::ObjectGet { object, .. } => vec![object],
            Instruction::ObjectSet { object, value, .. } => vec![object, value],
            Instruction::ClosureNew { code, captures, .. } => {
                let mut values = vec![code];
                values.extend(captures.iter().map(|(_, _, value)| value));
                values
            }
            Instruction::Alloc { .. }
            | Instruction::Load { .. }
            | Instruction::ArrayNew { .. }
            | Instruction::ObjectNew { .. } => Vec::new(),
        }
    }

    /// Mutable access to every value operand, for optimizer rewrites.
    pub fn values_mut(&mut self) -> Vec<&mut Value> {
        match self {
            Instruction::Store { value, .. }
            | Instruction::IsNull { value, .. }
            | Instruction::ToBool { value, .. } => vec![value],
            Instruction::Binary { lhs, rhs, .. }
            | Instruction::BigIntBinary { lhs, rhs, .. }
            | Instruction::StringConcat {
                left: lhs,
                right: rhs,
                ..
            }
            | Instruction::StringEquals {
                left: lhs,
                right: rhs,
                ..
            } => vec![lhs, rhs],
            Instruction::Unary { operand, .. }
            | Instruction::StringLength { value: operand, .. }
            | Instruction::NumberToString { value: operand, .. } => vec![operand],
            Instruction::StringCharAt { value, index, .. } => vec![value, index],
            Instruction::Call { args, .. } => args.iter_mut().collect(),
            Instruction::CallIndirect { callee, args, .. } => {
                let mut values = vec![callee];
                values.extend(args.iter_mut());
                values
            }
            Instruction::ArrayGet { array, index, .. } => vec![array, index],
            Instruction::ArraySet {
                array,
                index,
                value,
                ..
            } => vec![array, index, value],
            Instruction::ArrayLength { array, .. } => vec![array],
            Instruction::ObjectGet { object, .. } => vec![object],
            Instruction::ObjectSet { object, value, .. } => vec![object, value],
            Instruction::ClosureNew { code, captures, .. } => {
                let mut values = vec![code];
                values.extend(captures.iter_mut().map(|(_, _, value)| value));
                values
            }
            Instruction::Alloc { .. }
            | Instruction::Load { .. }
            | Instruction::ArrayNew { .. }
            | Instruction::ObjectNew { .. } => Vec::new(),
        }
    }

    /// The SSA value this instruction defines, if any.
    pub fn result(&self) -> Option<ValueId> {
        match self {
            Instruction::Load { result, .. }
            | Instruction::Binary { result, .. }
            | Instruction::Unary { result, .. }
            | Instruction::BigIntBinary { result, .. }
            | Instruction::IsNull { result, .. }
            | Instruction::ToBool { result, .. }
            | Instruction::StringConcat { result, .. }
            | Instruction::NumberToString { result, .. }
            | Instruction::StringEquals { result, .. }
            | Instruction::StringLength { result, .. }
            | Instruction::StringCharAt { result, .. }
            | Instruction::ArrayNew { result, .. }
            | Instruction::ArrayGet { result, .. }
            | Instruction::ArrayLength { result, .. }
            | Instruction::ObjectNew { result, .. }
            | Instruction::ObjectGet { result, .. }
            | Instruction::ClosureNew { result, .. } => Some(*result),
            Instruction::Call { result, .. } | Instruction::CallIndirect { result, .. } => *result,
            Instruction::Alloc { .. }
            | Instruction::Store { .. }
            | Instruction::ArraySet { .. }
            | Instruction::ObjectSet { .. } => None,
        }
    }

    /// Mutable access to the SSA value this instruction defines.
    pub fn result_mut(&mut self) -> Option<&mut ValueId> {
        match self {
            Instruction::Load { result, .. }
            | Instruction::Binary { result, .. }
            | Instruction::Unary { result, .. }
            | Instruction::BigIntBinary { result, .. }
            | Instruction::IsNull { result, .. }
            | Instruction::ToBool { result, .. }
            | Instruction::StringConcat { result, .. }
            | Instruction::NumberToString { result, .. }
            | Instruction::StringEquals { result, .. }
            | Instruction::StringLength { result, .. }
            | Instruction::StringCharAt { result, .. }
            | Instruction::ArrayNew { result, .. }
            | Instruction::ArrayGet { result, .. }
            | Instruction::ArrayLength { result, .. }
            | Instruction::ObjectNew { result, .. }
            | Instruction::ObjectGet { result, .. }
            | Instruction::ClosureNew { result, .. } => Some(result),
            Instruction::Call {
                result: Some(result),
                ..
            }
            | Instruction::CallIndirect {
                result: Some(result),
                ..
            } => Some(result),
            _ => None,
        }
    }

    /// The stack slot this instruction reads or writes, if any.
    pub fn slot(&self) -> Option<SlotId> {
        match self {
            Instruction::Alloc { slot, .. }
            | Instruction::Store { slot, .. }
            | Instruction::Load { slot, .. } => Some(*slot),
            _ => None,
        }
    }

    /// Mutable access to the stack slot this instruction reads or writes.
    pub fn slot_mut(&mut self) -> Option<&mut SlotId> {
        match self {
            Instruction::Alloc { slot, .. }
            | Instruction::Store { slot, .. }
            | Instruction::Load { slot, .. } => Some(slot),
            _ => None,
        }
    }

    /// Whether removing this instruction cannot change observable behaviour.
    /// Calls, allocations and fallible collection/runtime operations are never
    /// removable; a pure result with no uses is.
    pub fn is_removable(&self) -> bool {
        matches!(
            self,
            Instruction::Binary { .. }
                | Instruction::Unary { .. }
                | Instruction::BigIntBinary { .. }
                | Instruction::IsNull { .. }
                | Instruction::ToBool { .. }
                | Instruction::Load { .. }
        )
    }
}

impl Terminator {
    /// Every value operand of this terminator, in source order.
    pub fn values(&self) -> Vec<&Value> {
        match self {
            Terminator::Return(Some(value)) | Terminator::CondBr { cond: value, .. } => vec![value],
            _ => Vec::new(),
        }
    }

    /// Mutable access to every value operand, for optimizer rewrites.
    pub fn values_mut(&mut self) -> Vec<&mut Value> {
        match self {
            Terminator::Return(Some(value)) | Terminator::CondBr { cond: value, .. } => vec![value],
            _ => Vec::new(),
        }
    }
}
