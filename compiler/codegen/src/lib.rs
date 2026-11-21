//! Nava LLVM backend.
//!
//! Translation is deliberately textual: Nava IR → LLVM IR (`.ll`) → `llc` →
//! object → `clang` link. No LLVM Rust bindings are used (ADR-0004). The only
//! dependency is the IR crate, so the front end stays backend-independent.

use std::collections::HashMap;
use std::fmt::Write;

use nava_ir::{
    BinaryOp, Instruction, IrFunction, IrModule, IrType, Terminator, UnaryOp, Value, ValueId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct CodegenError {
    pub message: String,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// The runtime shim compiled alongside every generated program.
pub const RUNTIME_C: &str = include_str!("../../../runtime/nava_runtime.c");

const RUNTIME_DECLS: &[&str] = &[
    "declare void @nava_console_log_f64(double) nounwind",
    "declare void @nava_console_log_bool(i1) nounwind",
    "declare void @nava_console_log_str(ptr) nounwind",
    "declare void @nava_runtime_error(ptr) nounwind",
    "declare ptr @nava_string_concat(ptr, ptr) nounwind",
    "declare ptr @nava_number_to_string(double) nounwind",
    "declare i32 @nava_string_equals(ptr, ptr) nounwind",
    "declare i64 @nava_string_length(ptr) nounwind",
    "declare ptr @nava_string_char_at(ptr, i64) nounwind",
    "declare ptr @nava_array_new(i64) nounwind",
    "declare void @nava_array_set_f64(ptr, i64, double) nounwind",
    "declare double @nava_array_get_f64(ptr, i64) nounwind",
    "declare void @nava_array_set_bool(ptr, i64, i32) nounwind",
    "declare i32 @nava_array_get_bool(ptr, i64) nounwind",
    "declare void @nava_array_set_ptr(ptr, i64, ptr) nounwind",
    "declare ptr @nava_array_get_ptr(ptr, i64) nounwind",
    "declare i64 @nava_array_length(ptr) nounwind",
    "declare ptr @nava_object_new(i64, ptr) nounwind",
    "declare void @nava_object_set_f64(ptr, i64, double) nounwind",
    "declare double @nava_object_get_f64(ptr, i64) nounwind",
    "declare void @nava_object_set_bool(ptr, i64, i32) nounwind",
    "declare i32 @nava_object_get_bool(ptr, i64) nounwind",
    "declare void @nava_object_set_ptr(ptr, i64, ptr) nounwind",
    "declare ptr @nava_object_get_ptr(ptr, i64) nounwind",
];

fn escape_llvm_string(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\t' => out.push_str("\\09"),
            b'\r' => out.push_str("\\0D"),
            0x20..=0x7E => out.push(byte as char),
            other => {
                let _ = write!(out, "\\{other:02X}");
            }
        }
    }
    out
}

/// Generate textual LLVM IR for a verified Nava module.
pub fn generate(module: &IrModule, target_triple: &str) -> Result<String, CodegenError> {
    let mut out = String::new();
    let _ = writeln!(out, "target triple = \"{target_triple}\"\n");
    for declaration in RUNTIME_DECLS {
        let _ = writeln!(out, "{declaration}");
    }
    let _ = writeln!(out);

    for global in &module.globals {
        let text = escape_llvm_string(&global.value);
        let length = global.value.len() + 1;
        let _ = writeln!(
            out,
            "@g{} = private unnamed_addr constant [{length} x i8] c\"{text}\\00\"",
            global.id
        );
    }
    if !module.globals.is_empty() {
        let _ = writeln!(out);
    }

    // Object field-name tables are module-level constants; collect them before
    // emitting any function body.
    let mut object_globals: HashMap<String, String> = HashMap::new();
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Instruction::ObjectNew { fields, .. } = instruction {
                    let encoded = encode_field_names(fields);
                    if !object_globals.contains_key(&encoded) {
                        let name = format!("@obj_names_{}", object_globals.len());
                        object_globals.insert(encoded, name);
                    }
                }
            }
        }
    }
    for (encoded, name) in &object_globals {
        let byte_len = llvm_string_bytes(encoded);
        let _ = writeln!(
            out,
            "{name} = private unnamed_addr constant [{byte_len} x i8] c\"{encoded}\""
        );
    }
    if !object_globals.is_empty() {
        let _ = writeln!(out);
    }

    for function in &module.functions {
        // ValueIds are per-function: each function needs its own type map, or
        // ID collisions would give call arguments the wrong LLVM type.
        let types = value_types(function);
        emit_function(function, &types, &object_globals, &mut out)?;
    }
    if module.function("main").is_none() {
        out.push_str("define void @nava_main() {\n  ret void\n}\n\n");
    }
    Ok(out)
}

fn emit_function(
    function: &IrFunction,
    types: &HashMap<ValueId, IrType>,
    object_globals: &HashMap<String, String>,
    out: &mut String,
) -> Result<(), CodegenError> {
    let ret_ty = llvm_type(&function.return_ty).map_err(|message| CodegenError { message })?;
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            llvm_type(&param.ty)
                .map(|ty| format!("{ty} %p{index}"))
                .map_err(|message| CodegenError { message })
        })
        .collect::<Result<Vec<_>, _>>()?
        .join(", ");
    let _ = writeln!(out, "define {ret_ty} @nava_{}({params}) {{", function.name);

    for block in &function.blocks {
        let _ = writeln!(out, "bb{}:", block.id);
        for instruction in &block.instructions {
            emit_instruction(instruction, types, object_globals, out)?;
        }
        match &block.terminator {
            Some(Terminator::Return(None)) => {
                let _ = writeln!(out, "  ret void");
            }
            Some(Terminator::Return(Some(value))) => {
                let _ = writeln!(out, "  ret {} {}", ret_ty, llvm_value(value));
            }
            Some(Terminator::Br(target)) => {
                let _ = writeln!(out, "  br label %bb{target}");
            }
            Some(Terminator::CondBr {
                cond,
                then_block,
                else_block,
            }) => {
                let _ = writeln!(
                    out,
                    "  br i1 {}, label %bb{}, label %bb{}",
                    llvm_value(cond),
                    then_block,
                    else_block
                );
            }
            None => {
                return Err(CodegenError {
                    message: format!("block bb{} has no terminator", block.id),
                });
            }
        }
    }
    out.push_str("}\n\n");
    Ok(())
}

fn emit_instruction(
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

fn llvm_string_bytes(encoded: &str) -> usize {
    let chars: Vec<char> = encoded.chars().collect();
    let mut index = 0;
    let mut bytes = 0;
    while index < chars.len() {
        if chars[index] == '\\' && index + 2 < chars.len() {
            index += 3;
        } else {
            index += 1;
        }
        bytes += 1;
    }
    bytes
}

fn encode_field_names(fields: &[(String, IrType)]) -> String {
    let mut encoded = String::new();
    for (name, _) in fields {
        encoded.push_str(&escape_llvm_string(name));
        encoded.push_str("\\00");
    }
    encoded
}

fn slot_suffix(ty: &IrType) -> &'static str {
    match ty {
        IrType::Number => "f64",
        IrType::Boolean => "bool",
        _ => "ptr",
    }
}

fn index_arg(
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

fn bool_to_i32(value: &str, out: &mut String) -> String {
    let bool_temp = format!("bool{}", out.len());
    let int_temp = format!("int{}", out.len() + 1);
    let _ = writeln!(out, "  %{bool_temp} = add i1 {value}, false");
    let _ = writeln!(out, "  %{int_temp} = select i1 %{bool_temp}, i32 1, i32 0");
    format!("%{int_temp}")
}

fn require_type(actual: &IrType, expected: IrType, context: &str) -> Result<(), CodegenError> {
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

fn llvm_type(ty: &IrType) -> Result<String, String> {
    match ty {
        IrType::Number => Ok("double".to_owned()),
        IrType::Boolean => Ok("i1".to_owned()),
        IrType::Void => Ok("void".to_owned()),
        IrType::String | IrType::Array(_) | IrType::Object(_) => Ok("ptr".to_owned()),
        IrType::Opaque => Err("cannot lower an opaque type to LLVM".to_owned()),
    }
}

fn llvm_value(value: &Value) -> String {
    match value {
        Value::Param(index) => format!("%p{index}"),
        Value::Local(index) => format!("%v{index}"),
        Value::Number(value) => format!("{value:?}"),
        Value::Bool(value) => value.to_string(),
        Value::String(id) => format!("@g{id}"),
        Value::Null => "null".to_owned(),
    }
}

fn value_type(value: &Value, types: &HashMap<ValueId, IrType>) -> IrType {
    match value {
        Value::Number(_) => IrType::Number,
        Value::Bool(_) => IrType::Boolean,
        Value::String(_) | Value::Null => IrType::String,
        Value::Local(index) | Value::Param(index) => {
            types.get(index).cloned().unwrap_or(IrType::Opaque)
        }
    }
}

fn value_types(function: &IrFunction) -> HashMap<ValueId, IrType> {
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
                Instruction::ToBool { result, .. }
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
