//! Textual IR output for debugging and golden tests.

use std::fmt::Write;

use super::ir::{Instruction, IrFunction, IrModule, Terminator, Value};

/// Render a module in Nava's textual IR format.
pub fn print(module: &IrModule) -> String {
    let mut out = String::new();
    for global in &module.globals {
        let _ = writeln!(out, "global @{} = {:?}", global.id, global.value);
    }
    for function in &module.functions {
        print_function(function, &mut out);
    }
    out
}

fn print_function(function: &IrFunction, out: &mut String) {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| format!("%{index}: {}", param.ty.display()))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(
        out,
        "fn {}({}) -> {} {{",
        function.name,
        params,
        function.return_ty.display()
    );
    for block in &function.blocks {
        let _ = writeln!(out, "bb{} {}:", block.id, block.label);
        for instruction in &block.instructions {
            let _ = writeln!(out, "  {}", print_instruction(instruction));
        }
        match &block.terminator {
            Some(Terminator::Return(None)) => {
                let _ = writeln!(out, "  ret");
            }
            Some(Terminator::Return(Some(value))) => {
                let _ = writeln!(out, "  ret {}", value_name(value));
            }
            Some(Terminator::Br(target)) => {
                let _ = writeln!(out, "  br bb{target}");
            }
            Some(Terminator::CondBr {
                cond,
                then_block,
                else_block,
            }) => {
                let _ = writeln!(
                    out,
                    "  br {} ? bb{} : bb{}",
                    value_name(cond),
                    then_block,
                    else_block
                );
            }
            None => {
                let _ = writeln!(out, "  <missing terminator>");
            }
        }
    }
    out.push_str("}\n");
}

fn print_instruction(instruction: &Instruction) -> String {
    match instruction {
        Instruction::Alloc { slot, ty } => format!("${slot} = alloca {}", ty.display()),
        Instruction::Store { slot, ty, value } => {
            format!("store {}: {} -> ${slot}", value_name(value), ty.display())
        }
        Instruction::Load { result, slot, ty } => {
            format!("%{result} = load ${slot}: {}", ty.display())
        }
        Instruction::Binary {
            result,
            op,
            lhs,
            rhs,
        } => format!(
            "%{result} = {} {}, {}",
            op.symbol(),
            value_name(lhs),
            value_name(rhs)
        ),
        Instruction::BigIntBinary {
            result,
            op,
            lhs,
            rhs,
        } => format!(
            "%{result} = bigint.{} {}, {}",
            op.symbol(),
            value_name(lhs),
            value_name(rhs)
        ),
        Instruction::Unary {
            result,
            op,
            operand,
        } => {
            let symbol = match op {
                super::ir::UnaryOp::Neg | super::ir::UnaryOp::NegBigInt => "-",
                super::ir::UnaryOp::Not => "!",
            };
            format!("%{result} = {symbol}{}", value_name(operand))
        }
        Instruction::Call {
            result,
            callee,
            args,
            ..
        } => {
            let args = args.iter().map(value_name).collect::<Vec<_>>().join(", ");
            match result {
                Some(result) => format!("%{result} = call {callee}({args})"),
                None => format!("call {callee}({args})"),
            }
        }
        Instruction::IsNull { result, value } => {
            format!("%{result} = is_null {}", value_name(value))
        }
        Instruction::ToBool { result, value } => {
            format!("%{result} = bool {}", value_name(value))
        }
        Instruction::StringConcat {
            result,
            left,
            right,
        } => format!(
            "%{result} = string_concat {}, {}",
            value_name(left),
            value_name(right)
        ),
        Instruction::StringEquals {
            result,
            left,
            right,
        } => format!(
            "%{result} = string_eq {}, {}",
            value_name(left),
            value_name(right)
        ),
        Instruction::NumberToString { result, value } => {
            format!("%{result} = string {}", value_name(value))
        }
        Instruction::StringLength { result, value } => {
            format!("%{result} = string_length {}", value_name(value))
        }
        Instruction::StringCharAt {
            result,
            value,
            index,
        } => format!(
            "%{result} = string_char_at {}, {}",
            value_name(value),
            value_name(index)
        ),
        Instruction::ArrayNew {
            result,
            element_ty,
            length,
        } => format!("%{result} = array_new {} x {length}", element_ty.display()),
        Instruction::ArrayGet {
            result,
            array,
            index,
            ..
        } => format!(
            "%{result} = array_get {} [{}]",
            value_name(array),
            value_name(index)
        ),
        Instruction::ArraySet {
            array,
            index,
            value,
            ..
        } => format!(
            "array_set {} [{}] = {}",
            value_name(array),
            value_name(index),
            value_name(value)
        ),
        Instruction::ArrayLength { result, array } => {
            format!("%{result} = array_length {}", value_name(array))
        }
        Instruction::ObjectNew { result, fields } => format!(
            "%{result} = object_new {}",
            fields
                .iter()
                .map(|(name, ty)| format!("{name}: {}", ty.display()))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Instruction::ObjectGet {
            result,
            object,
            field_index,
            ..
        } => format!(
            "%{result} = object_get {} #{field_index}",
            value_name(object)
        ),
        Instruction::ObjectSet {
            object,
            field_index,
            value,
            ..
        } => format!(
            "object_set {} #{field_index} = {}",
            value_name(object),
            value_name(value)
        ),
    }
}

fn value_name(value: &Value) -> String {
    match value {
        Value::Param(index) => format!("%{index}"),
        Value::Local(index) => format!("%{index}"),
        Value::Number(value) => format!("{value}"),
        Value::BigInt(value) => format!("{value}n"),
        Value::Bool(value) => value.to_string(),
        Value::String(global) => format!("@{global}"),
        Value::Null => "null".to_owned(),
    }
}
