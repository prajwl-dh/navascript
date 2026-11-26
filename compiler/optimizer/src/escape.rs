//! Practical escape analysis: promote stack slots that never escape their
//! block into SSA values. Slots captured by closures or shared across blocks
//! are left alone, so memory layout only changes where it is provably safe.

use std::collections::HashMap;

use navascript_ir::{Instruction, IrFunction, IrModule, Value, ValueId};

use crate::utils::substitute;

/// A slot confined to one block with one store and loads after that store.
struct SlotShape {
    block: usize,
    stored: Value,
    loads: Vec<(usize, ValueId)>,
}

/// Promote non-escaping slots, returning the module and promotion count.
pub fn run(module: &IrModule) -> (IrModule, usize) {
    let mut output = module.clone();
    let mut promoted = 0;
    for function in &mut output.functions {
        promoted += promote(function);
    }
    (output, promoted)
}

fn promote(function: &mut IrFunction) -> usize {
    let slots: Vec<u32> = function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| instruction.slot())
        .collect();
    let mut promoted = 0;
    for slot in slots {
        let Some(shape) = slot_shape(function, slot) else {
            continue;
        };
        let replacements: HashMap<ValueId, Value> = shape
            .loads
            .iter()
            .map(|(_, id)| (*id, shape.stored.clone()))
            .collect();
        function.blocks[shape.block]
            .instructions
            .retain(|instruction| instruction.slot() != Some(slot));
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                for value in instruction.values_mut() {
                    substitute(value, &replacements);
                }
            }
            if let Some(terminator) = &mut block.terminator {
                for value in terminator.values_mut() {
                    substitute(value, &replacements);
                }
            }
        }
        promoted += 1;
    }
    promoted
}

/// A slot is promotable when one block allocates it, stores it exactly once,
/// and every load follows that store in the same block.
fn slot_shape(function: &IrFunction, slot: u32) -> Option<SlotShape> {
    let mut allocs = 0;
    let mut stores: Vec<(usize, usize, Value)> = Vec::new();
    let mut loads: Vec<(usize, usize, ValueId)> = Vec::new();

    for (block_index, block) in function.blocks.iter().enumerate() {
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            if instruction.slot() != Some(slot) {
                continue;
            }
            match instruction {
                Instruction::Alloc { .. } => allocs += 1,
                Instruction::Store { value, .. } => {
                    stores.push((block_index, instruction_index, value.clone()));
                }
                Instruction::Load { result, .. } => {
                    loads.push((block_index, instruction_index, *result));
                }
                _ => return None,
            }
        }
    }

    if allocs != 1 || stores.len() != 1 || loads.is_empty() {
        return None;
    }
    let (store_block, store_index, stored) = &stores[0];
    if loads
        .iter()
        .any(|(block, index, _)| block != store_block || index <= store_index)
    {
        return None;
    }
    if let Value::Local(id) = stored {
        if loads.iter().any(|(_, _, load_id)| load_id == id) {
            return None;
        }
    }
    Some(SlotShape {
        block: *store_block,
        stored: stored.clone(),
        loads: loads.iter().map(|(_, index, id)| (*index, *id)).collect(),
    })
}
