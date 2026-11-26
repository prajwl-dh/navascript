//! Shared IR analysis for the optimizer passes.

use std::collections::{HashMap, HashSet};

use navascript_ir::{Instruction, IrFunction, SlotId, Value, ValueId};

/// Value ids read anywhere in a function (as opposed to defined there).
pub fn used_values(function: &IrFunction) -> HashSet<ValueId> {
    let mut used = HashSet::new();
    for block in &function.blocks {
        for instruction in &block.instructions {
            for value in instruction.values() {
                if let Value::Local(id) = value {
                    used.insert(*id);
                }
            }
        }
        if let Some(terminator) = &block.terminator {
            for value in terminator.values() {
                if let Value::Local(id) = value {
                    used.insert(*id);
                }
            }
        }
    }
    used
}

/// One past the largest value id defined in a function.
pub fn next_value_id(function: &IrFunction) -> ValueId {
    function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter().filter_map(Instruction::result))
        .max()
        .unwrap_or(0)
        + 1
}

/// One past the largest slot id used in a function.
pub fn next_slot_id(function: &IrFunction) -> SlotId {
    function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            Instruction::Alloc { slot, .. }
            | Instruction::Store { slot, .. }
            | Instruction::Load { slot, .. } => Some(*slot),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        + 1
}

/// Replace local values according to a rewrite map, leaving other values alone.
pub fn substitute(value: &mut Value, replacements: &HashMap<ValueId, Value>) {
    if let Value::Local(id) = value {
        if let Some(replacement) = replacements.get(id) {
            *value = replacement.clone();
        }
    }
}
