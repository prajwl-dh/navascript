//! Verification helpers: value references, type requirements and slot checks.

use std::collections::{HashMap, HashSet};

use super::ir::{BlockId, Instruction, IrFunction, IrType, Terminator, Value, ValueId};
use super::verify::{VerifyError, error};

pub(super) fn values_in_instruction(instruction: &Instruction) -> Vec<&Value> {
    instruction.values()
}

pub(super) fn values_in_terminator(terminator: &Terminator) -> Vec<&Value> {
    terminator.values()
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
        Value::BigInt(_) => IrType::BigInt,
        Value::Bool(_) => IrType::Boolean,
        Value::String(_) => IrType::String,
        Value::Function { ty, .. } => (**ty).clone(),
        Value::Null => IrType::Opaque,
        Value::Param(index) | Value::Local(index) => defs
            .get(index)
            .map(|(ty, _)| ty.clone())
            .unwrap_or(IrType::Opaque),
    }
}
