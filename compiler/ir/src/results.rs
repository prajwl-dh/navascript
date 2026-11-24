//! Result types of IR instructions — one mapping for the verifier and the
//! backend, so the two can never silently disagree about a value's type.

use super::ir::{BinaryOp, Instruction, IrType, UnaryOp};

fn binary_result_type(op: &BinaryOp, bigint: bool) -> IrType {
    if op.is_comparison() || matches!(op, BinaryOp::And | BinaryOp::Or) {
        IrType::Boolean
    } else if bigint {
        IrType::BigInt
    } else {
        IrType::Number
    }
}

impl Instruction {
    /// The type of the value this instruction produces, or `Opaque` when it
    /// produces none.
    pub fn result_type(&self) -> IrType {
        match self {
            Instruction::Binary { op, .. } => binary_result_type(op, false),
            Instruction::BigIntBinary { op, .. } => binary_result_type(op, true),
            Instruction::Unary { op, .. } => match op {
                UnaryOp::Neg => IrType::Number,
                UnaryOp::NegBigInt => IrType::BigInt,
                UnaryOp::Not => IrType::Boolean,
            },
            Instruction::Load { ty, .. } => ty.clone(),
            Instruction::Call { result_ty, .. } | Instruction::CallIndirect { result_ty, .. } => {
                result_ty.clone()
            }
            Instruction::IsNull { .. }
            | Instruction::ToBool { .. }
            | Instruction::StringEquals { .. }
            | Instruction::NumberToString { .. } => IrType::Boolean,
            Instruction::StringLength { .. } | Instruction::ArrayLength { .. } => IrType::Number,
            Instruction::StringConcat { .. } | Instruction::StringCharAt { .. } => IrType::String,
            Instruction::ArrayNew { element_ty, .. } => IrType::Array(Box::new(element_ty.clone())),
            Instruction::ArrayGet { element_ty, .. }
            | Instruction::ObjectGet {
                field_ty: element_ty,
                ..
            } => element_ty.clone(),
            Instruction::ObjectNew { fields, .. } => IrType::Object(fields.clone()),
            Instruction::ClosureNew { fn_ty, .. } => fn_ty.clone(),
            _ => IrType::Opaque,
        }
    }
}
