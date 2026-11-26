//! Inline small, side-effect-free calls into their callers.

use std::collections::HashMap;

use navascript_ir::{Instruction, IrFunction, IrModule, Terminator, Value, ValueId};

use crate::utils::{next_slot_id, next_value_id};

/// Inline eligible calls, returning the new module and inlined-call count.
pub fn run(module: &IrModule) -> (IrModule, usize) {
    let eligible: HashMap<String, &IrFunction> = module
        .functions
        .iter()
        .filter(|function| is_eligible(function))
        .map(|function| (function.name.clone(), function))
        .collect();
    let mut output = module.clone();
    let mut inlined = 0;
    for function in &mut output.functions {
        inlined += inline_calls(function, &eligible);
    }
    (output, inlined)
}

/// A function is inlinable when it is one straight-line block, small, and has
/// no calls, closures or fallible/runtime operations of its own.
fn is_eligible(function: &IrFunction) -> bool {
    if function.blocks.len() != 1 || function.blocks[0].instructions.len() > 12 {
        return false;
    }
    let block = &function.blocks[0];
    if !matches!(
        block.terminator,
        Some(Terminator::Return(Some(Value::Local(_)))) | Some(Terminator::Return(None))
    ) {
        return false;
    }
    block.instructions.iter().all(|instruction| {
        !matches!(
            instruction,
            Instruction::Call { .. }
                | Instruction::CallIndirect { .. }
                | Instruction::ClosureNew { .. }
                | Instruction::StringConcat { .. }
                | Instruction::StringEquals { .. }
                | Instruction::StringLength { .. }
                | Instruction::StringCharAt { .. }
                | Instruction::ArrayGet { .. }
                | Instruction::ArraySet { .. }
                | Instruction::ArrayNew { .. }
                | Instruction::ObjectGet { .. }
                | Instruction::ObjectSet { .. }
                | Instruction::ObjectNew { .. }
        )
    })
}

fn inline_calls(function: &mut IrFunction, eligible: &HashMap<String, &IrFunction>) -> usize {
    let mut value_base = next_value_id(function);
    let mut slot_base = next_slot_id(function);
    let mut inlined = 0;

    for block in &mut function.blocks {
        let mut expanded = Vec::new();
        for instruction in block.instructions.drain(..) {
            let candidate = match &instruction {
                Instruction::Call {
                    result,
                    callee,
                    args,
                    ..
                } if callee != &function.name => eligible
                    .get(callee)
                    .map(|target| (*result, args.clone(), *target)),
                _ => None,
            };
            if let Some((result, args, target)) = candidate {
                if let Some(instructions) =
                    expand(target, result, &args, &mut value_base, &mut slot_base)
                {
                    expanded.extend(instructions);
                    inlined += 1;
                    continue;
                }
            }
            expanded.push(instruction);
        }
        block.instructions = expanded;
    }
    inlined
}

fn expand(
    target: &IrFunction,
    call_result: Option<ValueId>,
    args: &[Value],
    value_base: &mut ValueId,
    slot_base: &mut u32,
) -> Option<Vec<Instruction>> {
    let block = &target.blocks[0];
    let returned = match block.terminator {
        Some(Terminator::Return(Some(Value::Local(id)))) => Some(id),
        Some(Terminator::Return(None)) => None,
        _ => return None,
    };
    if returned.is_some() != call_result.is_some() {
        return None;
    }

    let mut params: HashMap<u32, Value> = HashMap::new();
    for (index, argument) in args.iter().enumerate() {
        params.insert(index as u32, argument.clone());
    }
    if params.len() != target.params.len() {
        return None;
    }

    // Map every callee result — including each copied instruction's own
    // `result` field — to a fresh id. The returned value takes over the
    // original call result so existing uses keep working. Renaming only the
    // operands would leave the call result undefined and the verifier would
    // reject the module.
    let mut values: HashMap<ValueId, ValueId> = HashMap::new();
    for instruction in &block.instructions {
        if let Some(id) = instruction.result() {
            let replacement = if Some(id) == returned {
                call_result.expect("checked above")
            } else {
                let fresh = *value_base;
                *value_base += 1;
                fresh
            };
            values.insert(id, replacement);
        }
    }

    let mut slots: HashMap<u32, u32> = HashMap::new();
    for instruction in &block.instructions {
        if let Some(slot) = instruction.slot() {
            slots.entry(slot).or_insert_with(|| {
                let fresh = *slot_base;
                *slot_base += 1;
                fresh
            });
        }
    }

    let mut expanded = Vec::new();
    for instruction in &block.instructions {
        let mut copy = instruction.clone();
        if let Some(result) = copy.result_mut() {
            if let Some(fresh) = values.get(result) {
                *result = *fresh;
            }
        }
        for value in copy.values_mut() {
            match value {
                Value::Local(id) => {
                    if let Some(fresh) = values.get(id) {
                        *value = Value::Local(*fresh);
                    }
                }
                Value::Param(index) => {
                    if let Some(argument) = params.get(index) {
                        *value = argument.clone();
                    }
                }
                _ => {}
            }
        }
        if let Some(slot) = copy.slot_mut() {
            if let Some(fresh) = slots.get(slot) {
                *slot = *fresh;
            }
        }
        expanded.push(copy);
    }
    Some(expanded)
}
