//! Statement and control-flow lowering into basic blocks.

use nava_ast::{Block, Expr, ExprKind, Stmt, StmtKind};

use super::builder::FunctionBuilder;
use super::ir::{BlockId, Instruction, IrType, Terminator, Value};
use super::lower::{Lowerer, ir_type, ir_type_from_symbol};

impl FunctionBuilder {
    pub(super) fn lower_block(
        &mut self,
        block: &Block,
        lowerer: &mut Lowerer,
        start: BlockId,
    ) -> Option<BlockId> {
        self.enter_scope();
        let mut current = Some(start);
        for statement in &block.statements {
            let Some(block_id) = current else {
                break;
            };
            current = self.lower_stmt(statement, lowerer, block_id);
        }
        self.leave_scope();
        current
    }

    pub(super) fn lower_stmt(
        &mut self,
        statement: &Stmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        self.current = Some(current);
        match &statement.kind {
            StmtKind::Return(value) => {
                let value = value.as_ref().map(|expr| self.lower_expr(expr, lowerer));
                self.set_terminator(current, Terminator::Return(value));
                None
            }
            StmtKind::Variable(decl) => {
                self.lower_variable(decl, lowerer);
                Some(current)
            }
            StmtKind::Expr(expr) => {
                self.lower_expr(expr, lowerer);
                Some(current)
            }
            StmtKind::If(if_stmt) => self.lower_if(if_stmt, lowerer, current),
            StmtKind::Block(block) => self.lower_block(block, lowerer, current),
            StmtKind::Empty => Some(current),
        }
    }

    pub(super) fn lower_variable(&mut self, decl: &nava_ast::VariableDecl, lowerer: &mut Lowerer) {
        let mut ty = ir_type(&decl.ty);
        if ty == IrType::Opaque {
            if let Some(init) = &decl.init {
                ty = lowerer
                    .typed
                    .types
                    .get(&init.span)
                    .map(ir_type_from_symbol)
                    .unwrap_or(IrType::Opaque);
            }
        }
        let slot = self.alloc(ty.clone());
        self.declare(&decl.name, slot);
        if let Some(init) = &decl.init {
            let value = self.lower_expr(init, lowerer);
            self.store(slot, ty, value);
        }
    }

    pub(super) fn lower_if(
        &mut self,
        if_stmt: &nava_ast::IfStmt,
        lowerer: &mut Lowerer,
        current: BlockId,
    ) -> Option<BlockId> {
        self.current = Some(current);
        let mut cond = self.lower_expr(&if_stmt.condition, lowerer);
        let cond_ty = lowerer
            .typed
            .types
            .get(&if_stmt.condition.span)
            .map(ir_type_from_symbol)
            .unwrap_or(IrType::Opaque);
        if cond_ty != IrType::Boolean && cond_ty != IrType::Opaque {
            let result = self.fresh_value();
            self.emit(Instruction::ToBool {
                result,
                value: cond,
            });
            cond = Value::Local(result);
        }
        let then_block = self.new_block("then");
        let else_block = self.new_block("else");
        let join_block = self.new_block("if.end");
        self.set_terminator(
            current,
            Terminator::CondBr {
                cond,
                then_block,
                else_block,
            },
        );

        let then_end = self.lower_block(&if_stmt.then_branch, lowerer, then_block);
        let else_end = match &if_stmt.else_branch {
            Some(nava_ast::ElseBranch::If(nested)) => self.lower_if(nested, lowerer, else_block),
            Some(nava_ast::ElseBranch::Block(block)) => {
                self.lower_block(block, lowerer, else_block)
            }
            None => Some(else_block),
        };

        match (then_end, else_end) {
            (None, None) => {
                self.function.remove_block(join_block);
                None
            }
            (Some(then_end), None) => {
                self.set_terminator(then_end, Terminator::Br(join_block));
                Some(join_block)
            }
            (None, Some(else_end)) => {
                self.set_terminator(else_end, Terminator::Br(join_block));
                Some(join_block)
            }
            (Some(then_end), Some(else_end)) => {
                self.set_terminator(then_end, Terminator::Br(join_block));
                self.set_terminator(else_end, Terminator::Br(join_block));
                Some(join_block)
            }
        }
    }

    pub(super) fn number_to_string(&mut self, value: Value) -> Value {
        let result = self.fresh_value();
        self.emit(Instruction::NumberToString { result, value });
        Value::Local(result)
    }

    pub(super) fn lower_assign(&mut self, target: &Expr, value: &Value, lowerer: &mut Lowerer) {
        match &target.kind {
            ExprKind::Ident(name) => match self.lookup(name) {
                Some((slot, ty)) => self.store(slot, ty, value.clone()),
                None => lowerer.error(format!("unknown value '{name}' in IR"), target.span),
            },
            ExprKind::Index { object, index } => {
                let array = self.lower_expr(object, lowerer);
                let index = self.lower_expr(index, lowerer);
                let element_ty = match lowerer.expr_ty(object) {
                    IrType::Array(inner) => *inner,
                    IrType::String => {
                        lowerer.error("cannot assign to a string index", target.span);
                        IrType::String
                    }
                    _ => {
                        lowerer.error("assignment requires an array", target.span);
                        IrType::Opaque
                    }
                };
                self.emit(Instruction::ArraySet {
                    array,
                    index,
                    value: value.clone(),
                    element_ty,
                });
            }
            ExprKind::Member { object, property } => {
                let object_ty = lowerer.expr_ty(object);
                match object_ty {
                    IrType::Object(fields) => {
                        if let Some((field_index, (_, field_ty))) = fields
                            .iter()
                            .enumerate()
                            .find(|(_, (name, _))| name == property)
                        {
                            let object = self.lower_expr(object, lowerer);
                            self.emit(Instruction::ObjectSet {
                                object,
                                field_index: field_index as u32,
                                value: value.clone(),
                                field_ty: field_ty.clone(),
                            });
                        } else {
                            lowerer.error(format!("unknown field '{property}'"), target.span);
                        }
                    }
                    _ => lowerer.error("assignment requires an object", target.span),
                }
            }
            _ => lowerer.error("unsupported assignment target in IR", target.span),
        }
    }
}
