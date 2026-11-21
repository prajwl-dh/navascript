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

/// Generate textual LLVM IR for a verified Nava module.
pub fn generate(module: &IrModule, target_triple: &str) -> Result<String, CodegenError> {
    let mut out = String::new();
    let _ = writeln!(out, "target triple = \"{target_triple}\"\n");
    let _ = writeln!(out, "declare void @nava_console_log_f64(double) nounwind");
    let _ = writeln!(out, "declare void @nava_console_log_bool(i1) nounwind\n");

    if !module.globals.is_empty() {
        return Err(CodegenError {
            message: "string globals are not supported by the native backend yet (Phase 5)"
                .to_owned(),
        });
    }

    for function in &module.functions {
        // ValueIds are per-function: each function needs its own type map, or
        // ID collisions would give call arguments the wrong LLVM type.
        let types = value_types(function);
        emit_function(function, &types, &mut out)?;
    }
    if module.function("main").is_none() {
        out.push_str("define void @nava_main() {\n  ret void\n}\n\n");
    }
    Ok(out)
}

fn emit_function(
    function: &IrFunction,
    types: &HashMap<ValueId, IrType>,
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
            emit_instruction(instruction, types, out)?;
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
        Instruction::Index { .. } | Instruction::Field { .. } => {
            return Err(CodegenError {
                message:
                    "field and element access are not supported by the native backend yet (Phase 5)"
                        .to_owned(),
            });
        }
    }
    Ok(())
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
        IrType::String => {
            Err("strings are not supported by the native backend yet (Phase 5)".to_owned())
        }
        IrType::Opaque => Err("cannot lower an opaque type to LLVM".to_owned()),
    }
}

fn llvm_value(value: &Value) -> String {
    match value {
        Value::Param(index) => format!("%p{index}"),
        Value::Local(index) => format!("%v{index}"),
        Value::Number(value) => format!("{value:?}"),
        Value::Bool(value) => value.to_string(),
        Value::String(_) | Value::Null => "null".to_owned(),
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
                Instruction::ToBool { result, .. } => {
                    types.insert(*result, IrType::Boolean);
                }
                Instruction::Call {
                    result: Some(result),
                    result_ty,
                    ..
                } => {
                    types.insert(*result, result_ty.clone());
                }
                Instruction::Index { result, .. } | Instruction::Field { result, .. } => {
                    types.insert(*result, IrType::Opaque);
                }
                _ => {}
            }
        }
    }
    types
}
