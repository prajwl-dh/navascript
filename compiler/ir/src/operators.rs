//! Lowering helpers for assignment, update and ternary operators.

use nava_ast::{AssignOp, BinaryOp as AstBinaryOp, Expr, UpdateOp};

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
        let delta = if op == UpdateOp::Inc { 1.0 } else { -1.0 };
        self.emit(Instruction::Binary {
            result,
            op: BinaryOp::Add,
            lhs: current.clone(),
            rhs: Value::Number(delta),
        });
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
