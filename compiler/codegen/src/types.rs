//! Type/value mapping helpers used by instruction emission.

use std::collections::HashMap;
use std::fmt::Write;

use nava_ir::{BinaryOp, Instruction, IrFunction, IrType, UnaryOp, Value, ValueId};

use super::CodegenError;

pub(super) fn slot_suffix(ty: &IrType) -> &'static str {
    match ty {
        IrType::Number => "f64",
        IrType::Boolean => "bool",
        _ => "ptr",
    }
}

pub(super) fn index_arg(
    value: &Value,
    types: &HashMap<ValueId, IrType>,
    out: &mut String,
) -> Result<String, CodegenError> {
    match value {
        Value::Number(index) if index.fract() == 0.0 => Ok(format!("{}", *index as i64)),
        Value::Number(_) => Err(CodegenError {
            message: "array/string index must be an integer".to_owned(),
        }),
        Value::Local(id) | Value::Param(id) => match types.get(id) {
            Some(IrType::Number) => {
                let temp = format!("idx{}", out.len());
                let _ = writeln!(
                    out,
                    "  %{temp} = fptosi double {} to i64",
                    llvm_value(value)
                );
                Ok(format!("%{temp}"))
            }
            Some(IrType::Boolean) => Err(CodegenError {
                message: "array/string index must be an integer".to_owned(),
            }),
            _ => Ok(llvm_value(value)),
        },
        _ => Err(CodegenError {
            message: "unsupported array/string index".to_owned(),
        }),
    }
}

pub(super) fn bool_to_i32(value: &str, out: &mut String) -> String {
    let bool_temp = format!("bool{}", out.len());
    let int_temp = format!("int{}", out.len() + 1);
    let _ = writeln!(out, "  %{bool_temp} = add i1 {value}, false");
    let _ = writeln!(out, "  %{int_temp} = select i1 %{bool_temp}, i32 1, i32 0");
    format!("%{int_temp}")
}

pub(super) fn require_type(
    actual: &IrType,
    expected: IrType,
    context: &str,
) -> Result<(), CodegenError> {
    if *actual != expected {
        return Err(CodegenError {
            message: format!(
                "{context} requires '{}', found '{}'",
                expected.display(),
                actual.display()
            ),
        });
    }
    Ok(())
}

pub(super) fn llvm_type(ty: &IrType) -> Result<String, String> {
    match ty {
        IrType::Number => Ok("double".to_owned()),
        IrType::Boolean => Ok("i1".to_owned()),
        IrType::Void => Ok("void".to_owned()),
        IrType::String | IrType::Array(_) | IrType::Object(_) => Ok("ptr".to_owned()),
        IrType::Opaque => Err("cannot lower an opaque type to LLVM".to_owned()),
    }
}

pub(super) fn llvm_value(value: &Value) -> String {
    match value {
        Value::Param(index) => format!("%p{index}"),
        Value::Local(index) => format!("%v{index}"),
        Value::Number(value) => format!("{value:?}"),
        Value::Bool(value) => value.to_string(),
        Value::String(id) => format!("@g{id}"),
        Value::Null => "null".to_owned(),
    }
}

pub(super) fn value_type(value: &Value, types: &HashMap<ValueId, IrType>) -> IrType {
    match value {
        Value::Number(_) => IrType::Number,
        Value::Bool(_) => IrType::Boolean,
        Value::String(_) | Value::Null => IrType::String,
        Value::Local(index) | Value::Param(index) => {
            types.get(index).cloned().unwrap_or(IrType::Opaque)
        }
    }
}

pub(super) fn value_types(function: &IrFunction) -> HashMap<ValueId, IrType> {
    let mut types = HashMap::new();
    for (index, param) in function.params.iter().enumerate() {
        types.insert(index as ValueId, param.ty.clone());
    }
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Load { result, ty, .. } => {
                    types.insert(*result, ty.clone());
                }
                Instruction::Binary { result, op, .. } => {
                    types.insert(
                        *result,
                        if op.is_comparison() || matches!(op, BinaryOp::And | BinaryOp::Or) {
                            IrType::Boolean
                        } else {
                            IrType::Number
                        },
                    );
                }
                Instruction::Unary { result, op, .. } => {
                    types.insert(
                        *result,
                        match op {
                            UnaryOp::Neg => IrType::Number,
                            UnaryOp::Not => IrType::Boolean,
                        },
                    );
                }
                Instruction::IsNull { result, .. }
                | Instruction::ToBool { result, .. }
                | Instruction::StringEquals { result, .. }
                | Instruction::NumberToString { result, .. } => {
                    types.insert(*result, IrType::Boolean);
                }
                Instruction::StringConcat { result, .. }
                | Instruction::StringCharAt { result, .. } => {
                    types.insert(*result, IrType::String);
                }
                Instruction::StringLength { result, .. }
                | Instruction::ArrayLength { result, .. } => {
                    types.insert(*result, IrType::Number);
                }
                Instruction::ArrayNew {
                    result, element_ty, ..
                } => {
                    types.insert(*result, IrType::Array(Box::new(element_ty.clone())));
                }
                Instruction::ArrayGet {
                    result, element_ty, ..
                }
                | Instruction::ObjectGet {
                    result,
                    field_ty: element_ty,
                    ..
                } => {
                    types.insert(*result, element_ty.clone());
                }
                Instruction::ObjectNew { result, fields } => {
                    types.insert(*result, IrType::Object(fields.clone()));
                }
                Instruction::Call {
                    result: Some(result),
                    result_ty,
                    ..
                } => {
                    types.insert(*result, result_ty.clone());
                }
                _ => {}
            }
        }
    }
    types
}
