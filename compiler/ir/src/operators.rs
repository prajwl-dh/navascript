//! Lowering helpers for assignment, update and ternary operators.

use navascript_ast::{AssignOp, BinaryOp as AstBinaryOp, Expr, Span, UpdateOp};

use super::builder::FunctionBuilder;
use super::ir::{BinaryOp, Instruction, IrType, Terminator, Value, ValueId};
use super::lower::Lowerer;

impl FunctionBuilder {
    /// Emit `+`, coercing a numeric operand to a string when the other is a
    /// string (JavaScript-style concatenation).
    pub(super) fn emit_add(
        &mut self,
        lhs: Value,
        rhs: Value,
        lhs_ty: IrType,
        rhs_ty: IrType,
        result: ValueId,
    ) {
        if lhs_ty == IrType::String || rhs_ty == IrType::String {
            let lhs = if lhs_ty == IrType::String {
                lhs
            } else {
                self.number_to_string(lhs)
            };
            let rhs = if rhs_ty == IrType::String {
                rhs
            } else {
                self.number_to_string(rhs)
            };
            self.emit(Instruction::StringConcat {
                result,
                left: lhs,
                right: rhs,
            });
        } else if lhs_ty == IrType::BigInt || rhs_ty == IrType::BigInt {
            self.emit(Instruction::BigIntBinary {
                result,
                op: BinaryOp::Add,
                lhs,
                rhs,
            });
        } else {
            self.emit(Instruction::Binary {
                result,
                op: BinaryOp::Add,
                lhs,
                rhs,
            });
        }
    }

    pub(super) fn lower_compound_assign(
        &mut self,
        op: AssignOp,
        target: &Expr,
        value: &Expr,
        lowerer: &mut Lowerer,
    ) -> Value {
        let current = self.lower_expr(target, lowerer);
        let operand = self.lower_expr(value, lowerer);
        let result = self.fresh_value();
        if op == AssignOp::Add {
            self.emit_add(
                current,
                operand,
                lowerer.expr_ty(target),
                lowerer.expr_ty(value),
                result,
            );
        } else if lowerer.expr_ty(target) == IrType::BigInt {
            self.emit(Instruction::BigIntBinary {
                result,
                op: to_ir_binary(op.binary_op()),
                lhs: current,
                rhs: operand,
            });
        } else {
            self.emit(Instruction::Binary {
                result,
                op: to_ir_binary(op.binary_op()),
                lhs: current,
                rhs: operand,
            });
        }
        let value = Value::Local(result);
        self.lower_assign(target, &value, lowerer);
        value
    }

    pub(super) fn lower_update(
        &mut self,
        op: UpdateOp,
        operand: &Expr,
        prefix: bool,
        lowerer: &mut Lowerer,
    ) -> Value {
        let current = self.lower_expr(operand, lowerer);
        let result = self.fresh_value();
        let increment = op == UpdateOp::Inc;
        if lowerer.expr_ty(operand) == IrType::BigInt {
            self.emit(Instruction::BigIntBinary {
                result,
                op: if increment {
                    BinaryOp::Add
                } else {
                    BinaryOp::Sub
                },
                lhs: current.clone(),
                rhs: Value::BigInt(1),
            });
        } else {
            let delta = if increment { 1.0 } else { -1.0 };
            self.emit(Instruction::Binary {
                result,
                op: BinaryOp::Add,
                lhs: current.clone(),
                rhs: Value::Number(delta),
            });
        }
        let updated = Value::Local(result);
        self.lower_assign(operand, &updated, lowerer);
        if prefix { updated } else { current }
    }

    /// Lower `cond ? a : b` to a result stack slot written in both branches.
    pub(super) fn lower_ternary(
        &mut self,
        condition: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
        ty: IrType,
        lowerer: &mut Lowerer,
    ) -> Value {
        let slot = self.alloc(ty.clone());
        let cond = self.condition_value(condition, lowerer);
        let current = self.current.expect("ternary expression outside a block");
        let then_block = self.new_block("ternary.then");
        let else_block = self.new_block("ternary.else");
        let join = self.new_block("ternary.end");
        self.set_terminator(
            current,
            Terminator::CondBr {
                cond,
                then_block,
                else_block,
            },
        );
        self.current = Some(then_block);
        let then_value = self.lower_expr(then_expr, lowerer);
        self.store(slot, ty.clone(), then_value);
        self.set_terminator(then_block, Terminator::Br(join));
        self.current = Some(else_block);
        let else_value = self.lower_expr(else_expr, lowerer);
        self.store(slot, ty.clone(), else_value);
        self.set_terminator(else_block, Terminator::Br(join));
        self.current = Some(join);
        self.load(slot, ty)
    }
}

impl FunctionBuilder {
    /// Lower `a?.b` / `a?.[i]` with a null check and a result stack slot.
    /// Only reference-typed results can represent `null` in the current ABI.
    pub(super) fn lower_optional_access(
        &mut self,
        access: OptionalAccess<'_>,
        lowerer: &mut Lowerer,
    ) -> Value {
        let OptionalAccess {
            object,
            target,
            object_ty,
            result_ty,
            span,
        } = access;
        let slot = self.alloc(result_ty.clone());
        let object_value = self.lower_expr(object, lowerer);
        let is_null = self.fresh_value();
        self.emit(Instruction::IsNull {
            result: is_null,
            value: object_value.clone(),
        });
        let null_block = self.new_block("optional.null");
        let value_block = self.new_block("optional.value");
        let join = self.new_block("optional.end");
        let current = self.current.expect("optional access outside a block");
        self.set_terminator(
            current,
            Terminator::CondBr {
                cond: Value::Local(is_null),
                then_block: null_block,
                else_block: value_block,
            },
        );

        self.current = Some(null_block);
        self.store(slot, result_ty.clone(), Value::Null);
        self.set_terminator(null_block, Terminator::Br(join));

        self.current = Some(value_block);
        let result = self.fresh_value();
        let index_value = match &target {
            OptionalTarget::Index(expr) => Some(self.lower_expr(expr, lowerer)),
            OptionalTarget::Member(_) => None,
        };
        match (&target, index_value, &object_ty) {
            (OptionalTarget::Index(_), Some(index), IrType::Array(element_ty)) => {
                self.emit(Instruction::ArrayGet {
                    result,
                    array: object_value,
                    index,
                    element_ty: *element_ty.clone(),
                });
            }
            (OptionalTarget::Index(_), Some(index), IrType::String) => {
                self.emit(Instruction::StringCharAt {
                    result,
                    value: object_value,
                    index,
                });
            }
            (OptionalTarget::Member(property), None, IrType::Object(fields)) => {
                if let Some((field_index, (_, field_ty))) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, (name, _))| name == property)
                {
                    self.emit(Instruction::ObjectGet {
                        result,
                        object: object_value,
                        field_index: field_index as u32,
                        field_ty: field_ty.clone(),
                    });
                } else {
                    lowerer.error(format!("unknown field '{property}'"), span);
                }
            }
            _ => lowerer.error("unsupported optional access target in IR", span),
        }
        self.store(slot, result_ty.clone(), Value::Local(result));
        self.set_terminator(value_block, Terminator::Br(join));

        self.current = Some(join);
        self.load(slot, result_ty)
    }
}

pub(super) enum OptionalTarget<'a> {
    Member(&'a str),
    Index(&'a Expr),
}

pub(super) struct OptionalAccess<'a> {
    pub object: &'a Expr,
    pub target: OptionalTarget<'a>,
    pub object_ty: IrType,
    pub result_ty: IrType,
    pub span: Span,
}

pub(super) fn to_ir_binary(op: AstBinaryOp) -> BinaryOp {
    match op {
        AstBinaryOp::Add => BinaryOp::Add,
        AstBinaryOp::Sub => BinaryOp::Sub,
        AstBinaryOp::Mul => BinaryOp::Mul,
        AstBinaryOp::Div => BinaryOp::Div,
        AstBinaryOp::Mod => BinaryOp::Mod,
        AstBinaryOp::Pow => BinaryOp::Pow,
        AstBinaryOp::Eq | AstBinaryOp::StrictEq => BinaryOp::Eq,
        AstBinaryOp::NotEq | AstBinaryOp::StrictNotEq => BinaryOp::Ne,
        AstBinaryOp::Lt => BinaryOp::Lt,
        AstBinaryOp::Le => BinaryOp::Le,
        AstBinaryOp::Gt => BinaryOp::Gt,
        AstBinaryOp::Ge => BinaryOp::Ge,
        AstBinaryOp::And => BinaryOp::And,
        AstBinaryOp::Or => BinaryOp::Or,
    }
}
