//! Loop lowering: control flow, scopes and iteration over arrays/strings.
//!
//! Every loop is expressed with basic-block back edges; local variables live in
//! stack slots allocated in the entry block (ADR-0013), so no phi nodes are
//! needed when a value crosses a loop header.

use nava_ast::{Expr, ExprKind, ForOfStmt, ForStmt, WhileStmt};

use super::builder::FunctionBuilder;
use super::ir::{BinaryOp, BlockId, Instruction, IrType, Terminator, Value};
use super::lower::Lowerer;

impl FunctionBuilder {
    /// Connect a loop body's fall-through block back to the header, if any.
    fn close_body(&mut self, body_end: Option<BlockId>, next: BlockId) {
        if let Some(end) = body_end {
            self.set_terminator(end, Terminator::Br(next));
        }
    }

    pub(super) fn lower_while(
        &mut self,
        stmt: &WhileStmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        let header = self.new_block("while.cond");
        let body = self.new_block("while.body");
        let exit = self.new_block("while.end");
        self.set_terminator(current, Terminator::Br(header));

        self.current = Some(header);
        let cond = self.condition_value(&stmt.condition, lowerer);
        self.set_terminator(
            header,
            Terminator::CondBr {
                cond,
                then_block: body,
                else_block: exit,
            },
        );

        let body_end = self.lower_block(&stmt.body, lowerer, body);
        if body_end.is_none() && matches!(stmt.condition.kind, ExprKind::Bool(true)) {
            // `while (true)` with a body that always returns cannot fall through.
            self.function.remove_block(exit);
            self.set_terminator(header, Terminator::Br(body));
            return None;
        }
        self.close_body(body_end, header);
        Some(exit)
    }

    pub(super) fn lower_do_while(
        &mut self,
        stmt: &WhileStmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        let body = self.new_block("do.body");
        let cond = self.new_block("do.cond");
        let exit = self.new_block("do.end");
        self.set_terminator(current, Terminator::Br(body));

        let Some(body_end) = self.lower_block(&stmt.body, lowerer, body) else {
            // The body always returns, so the loop can never repeat.
            self.function.remove_block(cond);
            self.function.remove_block(exit);
            return None;
        };
        self.set_terminator(body_end, Terminator::Br(cond));

        self.current = Some(cond);
        let condition = self.condition_value(&stmt.condition, lowerer);
        self.set_terminator(
            cond,
            Terminator::CondBr {
                cond: condition,
                then_block: body,
                else_block: exit,
            },
        );
        Some(exit)
    }

    pub(super) fn lower_for(
        &mut self,
        stmt: &ForStmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        self.enter_scope();
        self.current = Some(current);
        if let Some(init) = &stmt.init {
            self.lower_stmt(init, lowerer, current);
        }

        let header = self.new_block("for.cond");
        let body = self.new_block("for.body");
        let update = self.new_block("for.update");
        let exit = self.new_block("for.end");
        self.set_terminator(current, Terminator::Br(header));

        self.current = Some(header);
        let cond = match &stmt.condition {
            Some(expr) => self.condition_value(expr, lowerer),
            None => Value::Bool(true),
        };
        self.set_terminator(
            header,
            Terminator::CondBr {
                cond,
                then_block: body,
                else_block: exit,
            },
        );

        let body_end = self.lower_block(&stmt.body, lowerer, body);
        if body_end.is_none() && stmt.condition.is_none() {
            // `for (;;)` with a body that always returns cannot fall through.
            self.function.remove_block(update);
            self.function.remove_block(exit);
            self.set_terminator(header, Terminator::Br(body));
            self.leave_scope();
            return None;
        }
        self.close_body(body_end, update);
        self.current = Some(update);
        if let Some(expr) = &stmt.update {
            self.lower_expr(expr, lowerer);
        }
        self.set_terminator(update, Terminator::Br(header));

        self.leave_scope();
        Some(exit)
    }

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
        let body_end = self.lower_block(&stmt.body, lowerer, body);
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
