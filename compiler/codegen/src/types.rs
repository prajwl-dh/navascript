//! Type/value mapping helpers used by instruction emission.

use std::collections::HashMap;
use std::fmt::Write;

use navascript_ir::{Instruction, IrFunction, IrType, Value, ValueId};

use super::CodegenError;

pub(super) fn slot_suffix(ty: &IrType) -> &'static str {
    match ty {
        IrType::Number => "f64",
        IrType::BigInt => "i64",
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
                    "  %{temp} = call i64 @navascript_number_index(double {})",
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

/// Field layout of a closure object: code pointer first, then captures.
/// Shared by field-name collection and closure allocation so the two cannot
/// disagree about slot indices (ADR-0027).
pub(super) fn closure_fields(
    fn_ty: &IrType,
    captures: &[(String, IrType, Value)],
) -> Vec<(String, IrType)> {
    let mut fields = vec![("__code".to_owned(), fn_ty.clone())];
    fields.extend(
        captures
            .iter()
            .map(|(name, ty, _)| (name.clone(), ty.clone())),
    );
    fields
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
        IrType::BigInt => Ok("i64".to_owned()),
        IrType::Boolean => Ok("i1".to_owned()),
        IrType::Void => Ok("void".to_owned()),
        IrType::String | IrType::Array(_) | IrType::Object(_) | IrType::Function { .. } => {
            Ok("ptr".to_owned())
        }
        IrType::Opaque => Err("cannot lower an opaque type to LLVM".to_owned()),
    }
}

pub(super) fn llvm_value(value: &Value) -> String {
    match value {
        Value::Param(index) => format!("%p{index}"),
        Value::Local(index) => format!("%v{index}"),
        Value::Number(value) => format!("{value:?}"),
        Value::BigInt(value) => format!("{value}"),
        Value::Bool(value) => value.to_string(),
        Value::String(id) => format!("@g{id}"),
        Value::Function { name, .. } => format!("@{}", super::module::function_symbol(name)),
        Value::Null => "null".to_owned(),
    }
}

pub(super) fn value_type(value: &Value, types: &HashMap<ValueId, IrType>) -> IrType {
    match value {
        Value::Number(_) => IrType::Number,
        Value::BigInt(_) => IrType::BigInt,
        Value::Bool(_) => IrType::Boolean,
        Value::String(_) | Value::Null => IrType::String,
        Value::Function { ty, .. } => (**ty).clone(),
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
            let call_result = match instruction {
                Instruction::Call { result, .. } | Instruction::CallIndirect { result, .. } => {
                    *result
                }
                _ => None,
            };
            if let Some(result) = call_result {
                types.insert(result, instruction.result_type());
                continue;
            }
            let result = match instruction {
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
                | Instruction::ClosureNew { result, .. }
                | Instruction::ObjectGet { result, .. } => Some(result),
                _ => None,
            };
            if let Some(result) = result {
                types.insert(*result, instruction.result_type());
            }
        }
    }
    types
}
