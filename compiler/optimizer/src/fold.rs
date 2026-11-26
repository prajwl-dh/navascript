//! Constant folding: replace provably constant pure instructions with values.

use std::collections::HashMap;

use navascript_ir::{BinaryOp, IrFunction, IrModule, Terminator, UnaryOp, Value, ValueId};

use crate::utils::substitute;

/// Fold constants across a module, returning the new module and fold count.
pub fn run(module: &IrModule) -> (IrModule, usize) {
    let mut output = module.clone();
    let mut folded = 0;
    for function in &mut output.functions {
        folded += fold_function(function);
    }
    (output, folded)
}

fn fold_function(function: &mut IrFunction) -> usize {
    let mut replacements: HashMap<ValueId, Value> = HashMap::new();
    let mut folded = 0;

    // Fold in program order so later instructions see earlier replacements.
    for block in &mut function.blocks {
        let mut kept = Vec::new();
        for instruction in block.instructions.drain(..) {
            if let Some(result) = instruction.result() {
                if let Some(constant) = constant_result(&instruction, &replacements) {
                    replacements.insert(result, constant);
                    folded += 1;
                    continue;
                }
            }
            kept.push(instruction);
        }
        block.instructions = kept;
    }

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
            if let Terminator::CondBr {
                cond: Value::Bool(condition),
                then_block,
                else_block,
            } = terminator
            {
                let target = if *condition { *then_block } else { *else_block };
                *terminator = Terminator::Br(target);
                folded += 1;
            }
        }
    }
    folded
}

fn constant_result(
    instruction: &navascript_ir::Instruction,
    replacements: &HashMap<ValueId, Value>,
) -> Option<Value> {
    use navascript_ir::Instruction;
    let resolved = |value: &Value| -> Value {
        match value {
            Value::Local(id) => replacements
                .get(id)
                .cloned()
                .unwrap_or_else(|| value.clone()),
            other => other.clone(),
        }
    };
    match instruction {
        Instruction::Binary { op, lhs, rhs, .. } => {
            fold_binary(*op, &resolved(lhs), &resolved(rhs))
        }
        Instruction::BigIntBinary { op, lhs, rhs, .. } => {
            fold_binary(*op, &resolved(lhs), &resolved(rhs))
        }
        Instruction::Unary { op, operand, .. } => fold_unary(*op, &resolved(operand)),
        Instruction::IsNull { value, .. } => match resolved(value) {
            Value::Null => Some(Value::Bool(true)),
            Value::String(_) | Value::Function { .. } => Some(Value::Bool(false)),
            _ => None,
        },
        Instruction::ToBool { value, .. } => fold_to_bool(&resolved(value)),
        _ => None,
    }
}

fn fold_binary(op: BinaryOp, lhs: &Value, rhs: &Value) -> Option<Value> {
    match (lhs, rhs) {
        (Value::Number(a), Value::Number(b)) => match op {
            BinaryOp::Add => Some(Value::Number(a + b)),
            BinaryOp::Sub => Some(Value::Number(a - b)),
            BinaryOp::Mul => Some(Value::Number(a * b)),
            // Division/modulo by zero and `**` are left to LLVM so that its
            // exact IEEE semantics (and the pow intrinsic) remain authoritative.
            BinaryOp::Div if *b != 0.0 => Some(Value::Number(a / b)),
            BinaryOp::Mod if *b != 0.0 => Some(Value::Number(a % b)),
            BinaryOp::Eq => Some(Value::Bool(a == b)),
            BinaryOp::Ne => Some(Value::Bool(a != b)),
            BinaryOp::Lt => Some(Value::Bool(a < b)),
            BinaryOp::Le => Some(Value::Bool(a <= b)),
            BinaryOp::Gt => Some(Value::Bool(a > b)),
            BinaryOp::Ge => Some(Value::Bool(a >= b)),
            _ => None,
        },
        (Value::BigInt(a), Value::BigInt(b)) => match op {
            BinaryOp::Add => Some(Value::BigInt(a.wrapping_add(*b))),
            BinaryOp::Sub => Some(Value::BigInt(a.wrapping_sub(*b))),
            BinaryOp::Mul => Some(Value::BigInt(a.wrapping_mul(*b))),
            BinaryOp::Div if *b != 0 && !(*a == i64::MIN && *b == -1) => Some(Value::BigInt(a / b)),
            BinaryOp::Mod if *b != 0 && !(*a == i64::MIN && *b == -1) => Some(Value::BigInt(a % b)),
            BinaryOp::Eq => Some(Value::Bool(a == b)),
            BinaryOp::Ne => Some(Value::Bool(a != b)),
            BinaryOp::Lt => Some(Value::Bool(a < b)),
            BinaryOp::Le => Some(Value::Bool(a <= b)),
            BinaryOp::Gt => Some(Value::Bool(a > b)),
            BinaryOp::Ge => Some(Value::Bool(a >= b)),
            _ => None,
        },
        (Value::Bool(a), Value::Bool(b)) => match op {
            BinaryOp::Eq => Some(Value::Bool(a == b)),
            BinaryOp::Ne => Some(Value::Bool(a != b)),
            BinaryOp::And => Some(Value::Bool(*a && *b)),
            BinaryOp::Or => Some(Value::Bool(*a || *b)),
            _ => None,
        },
        _ => None,
    }
}

fn fold_unary(op: UnaryOp, operand: &Value) -> Option<Value> {
    match (op, operand) {
        (UnaryOp::Neg, Value::Number(value)) => Some(Value::Number(-value)),
        (UnaryOp::NegBigInt, Value::BigInt(value)) => Some(Value::BigInt(value.wrapping_neg())),
        (UnaryOp::Not, Value::Bool(value)) => Some(Value::Bool(!value)),
        _ => None,
    }
}

fn fold_to_bool(value: &Value) -> Option<Value> {
    match value {
        Value::Bool(value) => Some(Value::Bool(*value)),
        // The backend uses `fcmp one` and `icmp ne`, so NaN and zero are false.
        Value::Number(value) => Some(Value::Bool(!value.is_nan() && *value != 0.0)),
        Value::BigInt(value) => Some(Value::Bool(*value != 0)),
        _ => None,
    }
}
