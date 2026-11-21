//! Instruction-level LLVM IR emission.

use std::collections::HashMap;
use std::fmt::Write;

use nava_ir::{BinaryOp, Instruction, IrType, UnaryOp, Value, ValueId};

use super::CodegenError;
use super::runtime::encode_field_names;
use super::types::{bool_to_i32, index_arg, llvm_type, llvm_value, require_type};
use super::types::{slot_suffix, value_type};

pub(super) fn emit_instruction(
    instruction: &Instruction,
    types: &HashMap<ValueId, IrType>,
    object_globals: &HashMap<String, String>,
    out: &mut String,
) -> Result<(), CodegenError> {
    match instruction {
        Instruction::Alloc { slot, ty } => {
            let llvm = llvm_type(ty).map_err(|message| CodegenError { message })?;
            let _ = writeln!(out, "  %s{slot} = alloca {llvm}");
        }
        Instruction::Store { slot, ty, value } => {
            let llvm = llvm_type(ty).map_err(|message| CodegenError { message })?;
            let _ = writeln!(out, "  store {llvm} {}, ptr %s{slot}", llvm_value(value));
        }
        Instruction::Load { result, slot, ty } => {
            let llvm = llvm_type(ty).map_err(|message| CodegenError { message })?;
            let _ = writeln!(out, "  %v{result} = load {llvm}, ptr %s{slot}");
        }
        Instruction::Binary {
            result,
            op,
            lhs,
            rhs,
        } => {
            let lhs_ty = value_type(lhs, types);
            let rhs_ty = value_type(rhs, types);
            let lhs = llvm_value(lhs);
            let rhs = llvm_value(rhs);
            let line = match op {
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                    require_type(&lhs_ty, IrType::Number, "arithmetic")?;
                    require_type(&rhs_ty, IrType::Number, "arithmetic")?;
                    let name = match op {
                        BinaryOp::Add => "fadd",
                        BinaryOp::Sub => "fsub",
                        BinaryOp::Mul => "fmul",
                        BinaryOp::Div => "fdiv",
                        _ => "frem",
                    };
                    format!("{name} double {lhs}, {rhs}")
                }
                BinaryOp::Eq | BinaryOp::Ne => {
                    if lhs_ty == IrType::Boolean {
                        require_type(&rhs_ty, IrType::Boolean, "comparison")?;
                        let name = if *op == BinaryOp::Eq { "eq" } else { "ne" };
                        format!("icmp {name} i1 {lhs}, {rhs}")
                    } else {
                        require_type(&lhs_ty, IrType::Number, "comparison")?;
                        require_type(&rhs_ty, IrType::Number, "comparison")?;
                        let name = if *op == BinaryOp::Eq { "oeq" } else { "one" };
                        format!("fcmp {name} double {lhs}, {rhs}")
                    }
                }
                BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                    require_type(&lhs_ty, IrType::Number, "ordering comparison")?;
                    require_type(&rhs_ty, IrType::Number, "ordering comparison")?;
                    let name = match op {
                        BinaryOp::Lt => "olt",
                        BinaryOp::Le => "ole",
                        BinaryOp::Gt => "ogt",
                        _ => "oge",
                    };
                    format!("fcmp {name} double {lhs}, {rhs}")
                }
                BinaryOp::And | BinaryOp::Or => {
                    require_type(&lhs_ty, IrType::Boolean, "logical operator")?;
                    require_type(&rhs_ty, IrType::Boolean, "logical operator")?;
                    let name = if *op == BinaryOp::And { "and" } else { "or" };
                    format!("{name} i1 {lhs}, {rhs}")
                }
            };
            let _ = writeln!(out, "  %v{result} = {line}");
        }
        Instruction::Unary {
            result,
            op,
            operand,
        } => {
            let operand = llvm_value(operand);
            let line = match op {
                UnaryOp::Neg => format!("fneg double {operand}"),
                UnaryOp::Not => format!("xor i1 {operand}, true"),
            };
            let _ = writeln!(out, "  %v{result} = {line}");
        }
        Instruction::ToBool { result, value } => {
            let line = match value {
                Value::Bool(_) => format!("copy i1 {}", llvm_value(value)),
                Value::Number(_) => {
                    format!("fcmp one double {}, 0.0", llvm_value(value))
                }
                _ => {
                    let ty = value_type(value, types);
                    match ty {
                        IrType::Boolean => format!("copy i1 {}", llvm_value(value)),
                        IrType::Number => {
                            format!("fcmp one double {}, 0.0", llvm_value(value))
                        }
                        _ => {
                            return Err(CodegenError {
                                message: format!(
                                    "cannot convert value to bool in the native backend: {value:?}"
                                ),
                            });
                        }
                    }
                }
            };
            if let Some(rest) = line.strip_prefix("copy i1 ") {
                let _ = writeln!(out, "  %v{result} = add i1 {rest}, false");
            } else {
                let _ = writeln!(out, "  %v{result} = {line}");
            }
        }
        Instruction::Call {
            result,
            result_ty,
            callee,
            args,
        } => {
            let arg_tys: Vec<IrType> = args.iter().map(|arg| value_type(arg, types)).collect();
            let args = args
                .iter()
                .zip(&arg_tys)
                .map(|(arg, ty)| {
                    let llvm_ty = llvm_type(ty).map_err(|message| CodegenError { message })?;
                    Ok(format!("{llvm_ty} {}", llvm_value(arg)))
                })
                .collect::<Result<Vec<_>, CodegenError>>()?
                .join(", ");
            let (name, ret) = if callee == "console.log" {
                let arg_ty = arg_tys.first().cloned();
                let name = match arg_ty {
                    Some(IrType::Boolean) => "nava_console_log_bool",
                    Some(IrType::String) | Some(IrType::Array(_)) | Some(IrType::Object(_)) => {
                        "nava_console_log_str"
                    }
                    Some(IrType::Number) | None => "nava_console_log_f64",
                    Some(other) => {
                        return Err(CodegenError {
                            message: format!(
                                "console.log does not support '{}' yet",
                                other.display()
                            ),
                        });
                    }
                };
                (name.to_owned(), "void".to_owned())
            } else {
                (
                    format!("nava_{callee}"),
                    llvm_type(result_ty).map_err(|message| CodegenError { message })?,
                )
            };
            match result {
                Some(result) => {
                    let _ = writeln!(out, "  %v{result} = call {ret} @{name}({args})");
                }
                None => {
                    let _ = writeln!(out, "  call {ret} @{name}({args})");
                }
            }
        }
        Instruction::StringConcat {
            result,
            left,
            right,
        } => {
            let _ = writeln!(
                out,
                "  %v{result} = call ptr @nava_string_concat(ptr {}, ptr {})",
                llvm_value(left),
                llvm_value(right)
            );
        }
        Instruction::NumberToString { result, value } => {
            let _ = writeln!(
                out,
                "  %v{result} = call ptr @nava_number_to_string(double {})",
                llvm_value(value)
            );
        }
        Instruction::StringEquals {
            result,
            left,
            right,
        } => {
            let _ = writeln!(
                out,
                "  %raw{result} = call i32 @nava_string_equals(ptr {}, ptr {})",
                llvm_value(left),
                llvm_value(right)
            );
            let _ = writeln!(out, "  %v{result} = icmp ne i32 %raw{result}, 0");
        }
        Instruction::StringLength { result, value } => {
            let _ = writeln!(
                out,
                "  %len{result} = call i64 @nava_string_length(ptr {})",
                llvm_value(value)
            );
            let _ = writeln!(out, "  %v{result} = sitofp i64 %len{result} to double");
        }
        Instruction::StringCharAt {
            result,
            value,
            index,
        } => {
            let index = index_arg(index, types, out)?;
            let _ = writeln!(
                out,
                "  %v{result} = call ptr @nava_string_char_at(ptr {}, i64 {index})",
                llvm_value(value)
            );
        }
        Instruction::ArrayNew { result, length, .. } => {
            let _ = writeln!(out, "  %v{result} = call ptr @nava_array_new(i64 {length})");
        }
        Instruction::ArrayGet {
            result,
            array,
            index,
            element_ty,
        } => {
            let suffix = slot_suffix(element_ty);
            let llvm_ret = if *element_ty == IrType::Boolean {
                "i32".to_owned()
            } else {
                llvm_type(element_ty).map_err(|message| CodegenError { message })?
            };
            let index = index_arg(index, types, out)?;
            let _ = writeln!(
                out,
                "  %{}{} = call {} @nava_array_get_{}(ptr {}, i64 {index})",
                if *element_ty == IrType::Boolean {
                    "raw"
                } else {
                    "v"
                },
                result,
                llvm_ret,
                suffix,
                llvm_value(array)
            );
            if *element_ty == IrType::Boolean {
                let _ = writeln!(out, "  %v{result} = trunc i32 %raw{result} to i1");
            }
        }
        Instruction::ArraySet {
            array,
            index,
            value,
            element_ty,
        } => {
            let suffix = slot_suffix(element_ty);
            let (arg_ty, arg) = if *element_ty == IrType::Boolean {
                ("i32".to_owned(), bool_to_i32(&llvm_value(value), out))
            } else {
                (
                    llvm_type(element_ty).map_err(|message| CodegenError { message })?,
                    llvm_value(value),
                )
            };
            let index = index_arg(index, types, out)?;
            let _ = writeln!(
                out,
                "  call void @nava_array_set_{}(ptr {}, i64 {index}, {} {arg})",
                suffix,
                llvm_value(array),
                arg_ty
            );
        }
        Instruction::ArrayLength { result, array } => {
            let _ = writeln!(
                out,
                "  %len{result} = call i64 @nava_array_length(ptr {})",
                llvm_value(array)
            );
            let _ = writeln!(out, "  %v{result} = sitofp i64 %len{result} to double");
        }
        Instruction::ObjectNew { result, fields } => {
            let encoded = encode_field_names(fields);
            let names_global = object_globals.get(&encoded).ok_or_else(|| CodegenError {
                message: "missing object field-name global".to_owned(),
            })?;
            let _ = writeln!(
                out,
                "  %v{result} = call ptr @nava_object_new(i64 {}, ptr {names_global})",
                fields.len()
            );
        }
        Instruction::ObjectGet {
            result,
            object,
            field_index,
            field_ty,
        } => {
            let suffix = slot_suffix(field_ty);
            let llvm_ret = if *field_ty == IrType::Boolean {
                "i32".to_owned()
            } else {
                llvm_type(field_ty).map_err(|message| CodegenError { message })?
            };
            let _ = writeln!(
                out,
                "  %{}{} = call {} @nava_object_get_{}(ptr {}, i64 {})",
                if *field_ty == IrType::Boolean {
                    "raw"
                } else {
                    "v"
                },
                result,
                llvm_ret,
                suffix,
                llvm_value(object),
                field_index
            );
            if *field_ty == IrType::Boolean {
                let _ = writeln!(out, "  %v{result} = trunc i32 %raw{result} to i1");
            }
        }
        Instruction::ObjectSet {
            object,
            field_index,
            value,
            field_ty,
        } => {
            let suffix = slot_suffix(field_ty);
            let (arg_ty, arg) = if *field_ty == IrType::Boolean {
                ("i32".to_owned(), bool_to_i32(&llvm_value(value), out))
            } else {
                (
                    llvm_type(field_ty).map_err(|message| CodegenError { message })?,
                    llvm_value(value),
                )
            };
            let _ = writeln!(
                out,
                "  call void @nava_object_set_{}(ptr {}, i64 {}, {} {})",
                suffix,
                llvm_value(object),
                field_index,
                arg_ty,
                arg
            );
        }
    }
    Ok(())
}
