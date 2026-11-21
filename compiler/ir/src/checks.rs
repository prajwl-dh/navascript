//! Verification helpers: value references, type requirements and slot checks.

use std::collections::{HashMap, HashSet};

use super::ir::{BlockId, Instruction, IrFunction, IrType, Terminator, Value, ValueId};
use super::verify::{VerifyError, error};

pub(super) fn values_in_instruction(instruction: &Instruction) -> Vec<&Value> {
    match instruction {
        Instruction::Store { value, .. } | Instruction::ToBool { value, .. } => vec![value],
        Instruction::Binary { lhs, rhs, .. }
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
        Instruction::Alloc { .. }
        | Instruction::Load { .. }
        | Instruction::ArrayNew { .. }
        | Instruction::ObjectNew { .. } => Vec::new(),
    }
}

pub(super) fn values_in_terminator(terminator: &Terminator) -> Vec<&Value> {
    match terminator {
        Terminator::Return(Some(value)) => vec![value],
        Terminator::CondBr { cond, .. } => vec![cond],
        _ => Vec::new(),
    }
}

pub(super) fn require(
    actual: &IrType,
    expected: IrType,
    function: &IrFunction,
    block: BlockId,
    errors: &mut Vec<VerifyError>,
) {
    if *actual != IrType::Opaque && *actual != expected {
        errors.push(error(
            function,
            Some(block),
            format!(
                "expected '{}', found '{}'",
                expected.display(),
                actual.display()
            ),
        ));
    }
}

pub(super) fn check_slot(
    slot: &u32,
    ty: &IrType,
    available: &HashSet<u32>,
    slot_types: &HashMap<u32, (IrType, BlockId)>,
    function: &IrFunction,
    block: BlockId,
    errors: &mut Vec<VerifyError>,
) {
    if !available.contains(slot) {
        errors.push(error(
            function,
            Some(block),
            format!("stack slot ${slot} is not defined here"),
        ));
    }
    if let Some((declared, _)) = slot_types.get(slot) {
        if declared != ty {
            errors.push(error(
                function,
                Some(block),
                format!(
                    "slot ${slot} has type '{}', used as '{}'",
                    declared.display(),
                    ty.display()
                ),
            ));
        }
    }
}

pub(super) fn check_value(
    value: &Value,
    available: &HashSet<ValueId>,
    function: &IrFunction,
    block: BlockId,
    errors: &mut Vec<VerifyError>,
) {
    match value {
        Value::Local(id) => {
            if !available.contains(id) {
                errors.push(error(
                    function,
                    Some(block),
                    format!("value %{id} is not defined here"),
                ));
            }
        }
        Value::Param(index) if *index as usize >= function.params.len() => {
            errors.push(error(
                function,
                Some(block),
                format!("parameter %{index} is out of range"),
            ));
        }
        _ => {}
    }
}

pub(super) fn value_type(value: &Value, defs: &HashMap<ValueId, (IrType, BlockId)>) -> IrType {
    match value {
        Value::Number(_) => IrType::Number,
        Value::Bool(_) => IrType::Boolean,
        Value::String(_) => IrType::String,
        Value::Null => IrType::Opaque,
        Value::Param(index) | Value::Local(index) => defs
            .get(index)
            .map(|(ty, _)| ty.clone())
            .unwrap_or(IrType::Opaque),
    }
}

pub(super) fn instruction_type(instruction: &Instruction) -> IrType {
    match instruction {
        Instruction::Binary { op, .. } => {
            if op.is_comparison() {
                IrType::Boolean
            } else {
                IrType::Number
            }
        }
        Instruction::Unary { op, .. } => match op {
            super::ir::UnaryOp::Neg => IrType::Number,
            super::ir::UnaryOp::Not => IrType::Boolean,
        },
        Instruction::Load { ty, .. } => ty.clone(),
        Instruction::Call { result_ty, .. } => result_ty.clone(),
        Instruction::ToBool { .. } | Instruction::StringEquals { .. } => IrType::Boolean,
        Instruction::StringLength { .. } | Instruction::ArrayLength { .. } => IrType::Number,
        Instruction::StringConcat { .. }
        | Instruction::StringCharAt { .. }
        | Instruction::NumberToString { .. } => IrType::String,
        Instruction::ArrayNew { element_ty, .. } => IrType::Array(Box::new(element_ty.clone())),
        Instruction::ArrayGet { element_ty, .. } => element_ty.clone(),
        Instruction::ObjectNew { fields, .. } => IrType::Object(fields.clone()),
        Instruction::ObjectGet { field_ty, .. } => field_ty.clone(),
        _ => IrType::Opaque,
    }
}
