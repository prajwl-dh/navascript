//! Module-level emission: globals and function definitions.

use std::collections::HashMap;
use std::fmt::Write;

use navascript_ir::{Instruction, IrFunction, IrModule, IrType, Terminator, ValueId};

use super::CodegenError;
use super::instructions::emit_instruction;
use super::runtime::{RUNTIME_DECLS, encode_field_names, escape_llvm_string, llvm_string_bytes};
use super::types::{llvm_type, llvm_value, value_types};

pub(super) fn emit_module(module: &IrModule, target_triple: &str) -> Result<String, CodegenError> {
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
        out.push_str("define void @navascript_main() {\n  ret void\n}\n\n");
    }
    Ok(out)
}

pub(super) fn emit_function(
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
    let _ = writeln!(
        out,
        "define {ret_ty} @navascript_{}({params}) {{",
        function.name
    );

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
