//! Exception lowering for same-function `try`/`catch`/`finally` (ADR-0028).
//!
//! A `throw` stores its value in the enclosing try's slot and branches to the
//! catch block. Cross-function unwinding is not supported: `throw` without a
//! handler in the current function is diagnosed, so no exception is silently
//! lost. Because abrupt control flow out of protected regions is also
//! diagnosed, the normal and catch paths can converge cleanly.

use navascript_ast::{Block, Expr, Span, TryStmt};

use super::builder::{FunctionBuilder, TryContext};
use super::ir::{BlockId, IrType, Terminator};
use super::lower::{Lowerer, ir_type, ir_type_from_symbol};

impl FunctionBuilder {
    /// Lower `throw value` to a store plus a branch to the innermost catch.
    /// Returns `None` because the current block is terminated.
    pub(super) fn lower_throw(
        &mut self,
        expr: &Expr,
        span: Span,
        lowerer: &mut Lowerer,
    ) -> Option<BlockId> {
        let Some(context) = self.tries.last().cloned() else {
            lowerer.error("throw requires an enclosing try in the same function", span);
            return self.current;
        };
        self.tries
            .last_mut()
            .expect("try context present")
            .catch_used = true;
        let value = self.lower_expr(expr, lowerer);
        self.store(context.value_slot, context.value_ty, value);
        let end = self.current.expect("a throw always has a current block");
        self.set_terminator(end, Terminator::Br(context.catch_block));
        None
    }

    /// Lower a `try` statement into ordinary basic blocks: the body and catch
    /// both converge on `finally` (when present) and then on the join block.
    pub(super) fn lower_try(
        &mut self,
        stmt: &TryStmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        let value_ty = lowerer
            .typed
            .types
            .get(&stmt.catch_param.name.span)
            .map(|ty| ir_type_from_symbol(ty, &lowerer.current_subst))
            .filter(|ty| *ty != IrType::Opaque)
            .unwrap_or_else(|| ir_type(&stmt.catch_param.ty));
        let value_slot = self.alloc(value_ty.clone());
        let catch_block = self.new_block("catch");
        let finally_block = stmt
            .finally_body
            .as_ref()
            .map(|_| self.new_block("finally"));
        let after_block = self.new_block("try.end");

        self.tries.push(TryContext {
            value_slot,
            value_ty: value_ty.clone(),
            catch_block,
            catch_used: false,
        });
        let body_end = self.lower_block(&stmt.body, lowerer, current);
        let catch_used = self
            .tries
            .pop()
            .expect("try context is still on the stack")
            .catch_used;

        let convergence = finally_block.unwrap_or(after_block);
        if let Some(end) = body_end {
            self.set_terminator(end, Terminator::Br(convergence));
        }

        let catch_end = if catch_used {
            self.current = Some(catch_block);
            let param_slot = self.alloc(value_ty.clone());
            let thrown = self.load(value_slot, value_ty.clone());
            self.store(param_slot, value_ty.clone(), thrown);
            self.declare(&stmt.catch_param.name, param_slot);
            let end = self.lower_unprotected_block(&stmt.catch_body, lowerer, catch_block);
            if let Some(end) = end {
                self.set_terminator(end, Terminator::Br(convergence));
            }
            end
        } else {
            self.function.remove_block(catch_block);
            None
        };

        if let (Some(finally), Some(block)) = (&stmt.finally_body, finally_block) {
            let finally_end = self.lower_unprotected_block(finally, lowerer, block);
            if let Some(end) = finally_end {
                self.set_terminator(end, Terminator::Br(after_block));
            }
        }

        if body_end.is_none() && catch_end.is_none() && finally_block.is_none() {
            self.function.remove_block(after_block);
            return None;
        }
        self.current = Some(after_block);
        Some(after_block)
    }

    /// Lower catch/finally bodies with no active handler: a `throw` there
    /// would bypass the finally block, so it is diagnosed instead.
    fn lower_unprotected_block(
        &mut self,
        block: &Block,
        lowerer: &mut Lowerer,
        start: BlockId,
    ) -> Option<BlockId> {
        let outer = std::mem::take(&mut self.tries);
        let end = self.lower_block(block, lowerer, start);
        self.tries = outer;
        end
    }
}
