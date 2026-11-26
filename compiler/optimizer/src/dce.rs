//! Dead code elimination for pure instructions and unreachable blocks.

use std::collections::HashSet;

use navascript_ir::{IrFunction, IrModule, Terminator};

use crate::utils::used_values;

/// Remove unused pure instructions, returning the new module and removal count.
pub fn run(module: &IrModule) -> (IrModule, usize) {
    let mut output = module.clone();
    let mut eliminated = 0;
    for function in &mut output.functions {
        eliminated += eliminate(function);
    }
    (output, eliminated)
}

fn eliminate(function: &mut IrFunction) -> usize {
    let mut eliminated = 0;
    loop {
        let used = used_values(function);
        let mut removed = 0;
        for block in &mut function.blocks {
            block.instructions.retain(|instruction| {
                let dead = instruction.is_removable()
                    && instruction
                        .result()
                        .is_some_and(|value| !used.contains(&value));
                if dead {
                    removed += 1;
                }
                !dead
            });
        }
        eliminated += removed;
        eliminated += drop_unreachable_blocks(function);
        if removed == 0 {
            return eliminated;
        }
    }
}

/// Remove blocks that cannot be reached from the entry after branch folding.
fn drop_unreachable_blocks(function: &mut IrFunction) -> usize {
    let mut reachable = HashSet::new();
    let mut pending = vec![function.entry];
    while let Some(id) = pending.pop() {
        if !reachable.insert(id) {
            continue;
        }
        let Some(block) = function.block(id) else {
            continue;
        };
        match &block.terminator {
            Some(Terminator::Br(target)) => pending.push(*target),
            Some(Terminator::CondBr {
                then_block,
                else_block,
                ..
            }) => {
                pending.push(*then_block);
                pending.push(*else_block);
            }
            _ => {}
        }
    }
    let before = function.blocks.len();
    function
        .blocks
        .retain(|block| reachable.contains(&block.id));
    before - function.blocks.len()
}
