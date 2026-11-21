//! `for (let x of xs)` lowering into an index-driven loop.

use nava_ast::{Expr, ExprKind, ForOfStmt};

use super::builder::FunctionBuilder;
use super::ir::{BinaryOp, BlockId, Instruction, IrType, Terminator, Value};
use super::lower::Lowerer;

impl FunctionBuilder {
    /// Lower `for (let x of xs)` into an index-driven loop over a stack slot.
    pub(super) fn lower_for_of(
        &mut self,
        stmt: &ForOfStmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        self.enter_scope();
        self.current = Some(current);

        let iterable_ty = lowerer.expr_ty(&stmt.iterable);
        let (element_ty, is_string) = match &iterable_ty {
            IrType::Array(inner) => ((**inner).clone(), false),
            IrType::String => (IrType::String, true),
            _ => {
                lowerer.error(
                    "for-of requires an array or string in IR",
                    stmt.iterable.span,
                );
                (IrType::Opaque, false)
            }
        };

        let iterable = self.lower_expr(&stmt.iterable, lowerer);
        let length = self.fresh_value();
        if is_string {
            self.emit(Instruction::StringLength {
                result: length,
                value: iterable.clone(),
            });
        } else {
            self.emit(Instruction::ArrayLength {
                result: length,
                array: iterable.clone(),
            });
        }

        let index_slot = self.alloc(IrType::Number);
        self.store(index_slot, IrType::Number, Value::Number(0.0));
        let binding_slot = stmt.decl.then(|| {
            let slot = self.alloc(element_ty.clone());
            self.declare(&stmt.binding, slot);
            slot
        });

        let header = self.new_block("forof.cond");
        let body = self.new_block("forof.body");
        let update = self.new_block("forof.update");
        let exit = self.new_block("forof.end");
        self.set_terminator(current, Terminator::Br(header));

        self.current = Some(header);
        let index = self.load(index_slot, IrType::Number);
        let less = self.fresh_value();
        self.emit(Instruction::Binary {
            result: less,
            op: BinaryOp::Lt,
            lhs: index,
            rhs: Value::Local(length),
        });
        self.set_terminator(
            header,
            Terminator::CondBr {
                cond: Value::Local(less),
                then_block: body,
                else_block: exit,
            },
        );

        self.current = Some(body);
        let index = self.load(index_slot, IrType::Number);
        let element = self.fresh_value();
        if is_string {
            self.emit(Instruction::StringCharAt {
                result: element,
                value: iterable,
                index,
            });
        } else {
            self.emit(Instruction::ArrayGet {
                result: element,
                array: iterable,
                index,
                element_ty: element_ty.clone(),
            });
        }
        if let Some(slot) = binding_slot {
            self.store(slot, element_ty, Value::Local(element));
        } else {
            let target = Expr {
                kind: ExprKind::Ident(stmt.binding.name.clone()),
                span: stmt.binding.span,
            };
            self.lower_assign(&target, &Value::Local(element), lowerer);
        }
        let (body_end, _) = self.with_loop(exit, update, &stmt.body, body, lowerer);
        self.close_body(body_end, update);

        self.current = Some(update);
        let index = self.load(index_slot, IrType::Number);
        let next = self.fresh_value();
        self.emit(Instruction::Binary {
            result: next,
            op: BinaryOp::Add,
            lhs: index,
            rhs: Value::Number(1.0),
        });
        self.store(index_slot, IrType::Number, Value::Local(next));
        self.set_terminator(update, Terminator::Br(header));

        self.leave_scope();
        Some(exit)
    }
}
