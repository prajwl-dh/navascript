//! Loop lowering: control flow, scopes and iteration over arrays/strings.
//!
//! Every loop is expressed with basic-block back edges; local variables live in
//! stack slots allocated in the entry block (ADR-0013), so no phi nodes are
//! needed when a value crosses a loop header.

use nava_ast::{Block, Expr, ExprKind, ForStmt, Stmt, WhileStmt};

use super::builder::{FunctionBuilder, LoopContext};
use super::ir::{BlockId, Terminator, Value};
use super::lower::Lowerer;

impl FunctionBuilder {
    /// Connect a loop body's fall-through block back to the header, if any.
    pub(super) fn close_body(&mut self, body_end: Option<BlockId>, next: BlockId) {
        if let Some(end) = body_end {
            self.set_terminator(end, Terminator::Br(next));
        }
    }

    /// Lower a loop body inside a pushed loop context, returning its tail block
    /// and whether a loop jump needs an exit/target block kept alive.
    pub(super) fn with_loop(
        &mut self,
        break_target: BlockId,
        continue_target: BlockId,
        body: &Block,
        body_start: BlockId,
        lowerer: &mut Lowerer,
    ) -> (Option<BlockId>, bool) {
        self.loops.push(LoopContext {
            break_target,
            continue_target,
            has_jump: false,
        });
        let body_end = self.lower_block(body, lowerer, body_start);
        let has_jump = self.loops.last().is_some_and(|ctx| ctx.has_jump);
        self.loops.pop();
        (body_end, has_jump)
    }

    /// Emit the post-body condition of a `do` loop into `cond_block`.
    fn emit_do_cond(
        &mut self,
        cond_block: BlockId,
        body: BlockId,
        exit: BlockId,
        condition: &Expr,
        lowerer: &mut Lowerer,
    ) {
        self.current = Some(cond_block);
        let value = self.condition_value(condition, lowerer);
        self.set_terminator(
            cond_block,
            Terminator::CondBr {
                cond: value,
                then_block: body,
                else_block: exit,
            },
        );
    }

    pub(super) fn lower_break_or_continue(
        &mut self,
        statement: &Stmt,
        lowerer: &mut Lowerer,
        fallback: BlockId,
    ) -> Option<BlockId> {
        let block = self.current.unwrap_or(fallback);
        let is_break = matches!(statement.kind, nava_ast::StmtKind::Break);
        let target = self.loops.last().map(|ctx| {
            if is_break {
                ctx.break_target
            } else {
                ctx.continue_target
            }
        });
        let Some(target) = target else {
            let word: &'static str = if is_break { "break" } else { "continue" };
            lowerer.error(format!("{word} outside a loop in IR"), statement.span);
            return Some(block);
        };
        self.set_terminator(block, Terminator::Br(target));
        if let Some(ctx) = self.loops.last_mut() {
            ctx.has_jump = true;
        }
        None
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

        let (body_end, has_jumps) = self.with_loop(exit, header, &stmt.body, body, lowerer);
        if body_end.is_none() && !has_jumps && matches!(stmt.condition.kind, ExprKind::Bool(true)) {
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

        let (body_result, has_jumps) = self.with_loop(exit, cond, &stmt.body, body, lowerer);
        let Some(body_end) = body_result else {
            if !has_jumps {
                // The body always returns, so the loop can never repeat.
                self.function.remove_block(cond);
                self.function.remove_block(exit);
                return None;
            }
            // A `break` can still reach the exit, so keep both blocks alive.
            self.emit_do_cond(cond, body, exit, &stmt.condition, lowerer);
            return Some(exit);
        };
        self.set_terminator(body_end, Terminator::Br(cond));

        self.emit_do_cond(cond, body, exit, &stmt.condition, lowerer);
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

        let (body_end, has_jumps) = self.with_loop(exit, update, &stmt.body, body, lowerer);
        if body_end.is_none() && !has_jumps && stmt.condition.is_none() {
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
}
