//! AST → IR lowering. All control flow, scoping and slot allocation decisions
//! are hidden behind `lower`.

use std::collections::HashMap;

use nava_ast::{
    BinaryOp as AstBinaryOp, Block, Expr, ExprKind, Ident, ItemKind, Program, Span, Stmt, StmtKind,
    TypeAnnotation, TypeExpr, UnaryOp as AstUnaryOp,
};
use nava_diagnostics::{Diagnostic, Diagnostics, LineIndex};
use nava_symbol::Type;
use nava_typechecker::TypedProgram;

use super::ir::{
    BasicBlock, BinaryOp, BlockId, GlobalId, Instruction, IrFunction, IrGlobal, IrModule, IrParam,
    IrType, SlotId, Terminator, UnaryOp, Value, ValueId,
};

/// The result of lowering: an owned module plus diagnostics in the same shape
/// as every earlier front-end phase.
#[derive(Debug, Clone, PartialEq)]
pub struct LowerOutput {
    pub module: IrModule,
    pub diagnostics: Diagnostics,
}

/// Lower a typed program into platform-independent IR.
///
/// Infallible: unsupported constructs become diagnostics, never panics. The
/// module is verified before it is returned; verifier errors are reported as
/// diagnostics too.
pub fn lower(program: &Program, typed: &TypedProgram, source: &str) -> LowerOutput {
    let mut lowerer = Lowerer {
        module: IrModule::default(),
        diagnostics: Vec::new(),
        lines: LineIndex::new(source),
        typed,
        next_global: 0,
    };
    lowerer.lower_program(program);
    for error in super::verify::verify(&lowerer.module) {
        lowerer.diagnostics.push(Diagnostic::error(
            error.to_string(),
            error.span.unwrap_or(program.span),
            &lowerer.lines,
        ));
    }
    LowerOutput {
        module: lowerer.module,
        diagnostics: Diagnostics::new(lowerer.diagnostics),
    }
}

struct Lowerer<'a> {
    module: IrModule,
    diagnostics: Vec<Diagnostic>,
    lines: LineIndex,
    typed: &'a TypedProgram,
    next_global: GlobalId,
}

impl<'a> Lowerer<'a> {
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message, span, &self.lines));
    }

    fn lower_program(&mut self, program: &Program) {
        let mut main_body = Vec::new();
        for item in &program.items {
            match &item.kind {
                ItemKind::Function(decl) => self.lower_function(decl),
                ItemKind::Variable(_) | ItemKind::Expr(_) => main_body.push(item),
                ItemKind::TypeAlias(_) | ItemKind::Interface(_) => {}
            }
        }
        if !main_body.is_empty() {
            self.lower_main(&main_body);
        }
    }

    fn lower_main(&mut self, items: &[&nava_ast::Item]) {
        let mut builder = FunctionBuilder::new("main".to_owned(), Vec::new(), IrType::Void);
        for item in items {
            match &item.kind {
                ItemKind::Variable(decl) => builder.lower_variable(decl, self),
                ItemKind::Expr(expr) => {
                    builder.lower_expr(expr, self);
                }
                _ => {}
            }
        }
        if let Some(current) = builder.current {
            builder.set_terminator(current, Terminator::Return(None));
        }
        self.module.functions.push(builder.finish());
    }

    fn lower_function(&mut self, decl: &nava_ast::FunctionDecl) {
        let params: Vec<IrParam> = decl
            .params
            .iter()
            .map(|param| IrParam {
                name: param.name.name.clone(),
                ty: ir_type(&param.ty),
            })
            .collect();
        let return_ty = if decl.return_ty.is_some() {
            ir_type_opt(&decl.return_ty)
        } else {
            self.typed
                .symbols
                .lookup(&decl.name.name)
                .map(|symbol| match &symbol.ty {
                    Type::Function { ret, .. } => ir_type_from_symbol(ret),
                    _ => IrType::Opaque,
                })
                .unwrap_or(IrType::Opaque)
        };
        let mut builder = FunctionBuilder::new(decl.name.name.clone(), params, return_ty.clone());

        let entry = builder.function.entry;
        for (index, param) in decl.params.iter().enumerate() {
            let ty = ir_type(&param.ty);
            let slot = builder.alloc(ty.clone());
            builder.store(slot, ty, Value::Param(index as u32));
            builder.declare(&param.name, slot);
        }

        let end = builder.lower_block(&decl.body, self, entry);
        if let Some(block) = end {
            let value = if return_ty == IrType::Void {
                None
            } else {
                self.error(
                    format!(
                        "function '{}' may fall through without returning",
                        decl.name.name
                    ),
                    decl.body.span,
                );
                Some(Value::Null)
            };
            builder.set_terminator(block, Terminator::Return(value));
        }
        self.module.functions.push(builder.finish());
    }

    fn string_global(&mut self, value: &str) -> GlobalId {
        let id = self.next_global;
        self.next_global += 1;
        self.module.globals.push(IrGlobal {
            id,
            value: value.to_owned(),
        });
        id
    }

    fn return_type_of(&self, name: &str) -> IrType {
        match self.typed.symbols.lookup(name).map(|symbol| &symbol.ty) {
            Some(Type::Function { ret, .. }) => ir_type_from_symbol(ret),
            Some(Type::Void) => IrType::Void,
            _ => IrType::Opaque,
        }
    }

    fn is_known_builtin(name: &str) -> bool {
        matches!(name, "console.log")
    }

    fn expr_ty(&self, expr: &Expr) -> IrType {
        self.typed
            .types
            .get(&expr.span)
            .map(ir_type_from_symbol)
            .unwrap_or(IrType::Opaque)
    }
}

fn ir_type(annotation: &Option<TypeAnnotation>) -> IrType {
    ir_type_opt(annotation)
}

fn ir_type_opt(annotation: &Option<TypeAnnotation>) -> IrType {
    annotation
        .as_ref()
        .map(|annotation| ir_type_expr(&annotation.expr))
        .unwrap_or(IrType::Opaque)
}

fn ir_type_expr(expr: &TypeExpr) -> IrType {
    match expr {
        TypeExpr::Named(name) => match name.as_str() {
            "number" => IrType::Number,
            "string" => IrType::String,
            "boolean" => IrType::Boolean,
            "void" => IrType::Void,
            _ => IrType::Opaque,
        },
        TypeExpr::Array(_) | TypeExpr::Object(_) | TypeExpr::Function { .. } => IrType::Opaque,
    }
}

fn ir_type_from_symbol(ty: &Type) -> IrType {
    match ty {
        Type::Number => IrType::Number,
        Type::String => IrType::String,
        Type::Boolean => IrType::Boolean,
        Type::Void => IrType::Void,
        Type::Array(inner) => IrType::Array(Box::new(ir_type_from_symbol(inner))),
        Type::Object(fields) => IrType::Object(
            fields
                .iter()
                .map(|(name, field_ty)| (name.clone(), ir_type_from_symbol(field_ty)))
                .collect(),
        ),
        _ => IrType::Opaque,
    }
}

/// Per-function builder. Branches, slots and values are all local to it.
struct FunctionBuilder {
    function: IrFunction,
    current: Option<BlockId>,
    next_value: ValueId,
    next_slot: SlotId,
    next_block: BlockId,
    scopes: Vec<HashMap<String, SlotId>>,
}

impl FunctionBuilder {
    fn new(name: String, params: Vec<IrParam>, return_ty: IrType) -> Self {
        let entry = 0;
        let next_value = params.len() as ValueId;
        Self {
            function: IrFunction {
                name,
                params,
                return_ty,
                entry,
                blocks: vec![BasicBlock::new(entry, "entry")],
            },
            current: Some(entry),
            next_value,
            next_slot: 0,
            next_block: 1,
            scopes: vec![HashMap::new()],
        }
    }

    fn finish(self) -> IrFunction {
        self.function
    }

    fn fresh_value(&mut self) -> ValueId {
        let value = self.next_value;
        self.next_value += 1;
        value
    }

    fn new_block(&mut self, label: &str) -> BlockId {
        let id = self.next_block;
        self.next_block += 1;
        self.function
            .blocks
            .push(BasicBlock::new(id, format!("{label}.{id}")));
        id
    }

    fn set_terminator(&mut self, block: BlockId, terminator: Terminator) {
        if let Some(block) = self.function.block_mut(block) {
            block.terminator = Some(terminator);
        }
    }

    fn emit_in(&mut self, block: BlockId, instruction: Instruction) {
        if let Some(block) = self.function.block_mut(block) {
            block.instructions.push(instruction);
        }
    }

    fn emit(&mut self, instruction: Instruction) {
        if let Some(id) = self.current {
            self.emit_in(id, instruction);
        }
    }

    /// Stack slots always live in the entry block so LLVM's `mem2reg` can
    /// promote them, regardless of where the variable was declared.
    fn alloc(&mut self, ty: IrType) -> SlotId {
        let slot = self.next_slot;
        self.next_slot += 1;
        let entry = self.function.entry;
        self.emit_in(entry, Instruction::Alloc { slot, ty });
        slot
    }

    fn store(&mut self, slot: SlotId, ty: IrType, value: Value) {
        self.emit(Instruction::Store { slot, ty, value });
    }

    fn load(&mut self, slot: SlotId, ty: IrType) -> Value {
        let result = self.fresh_value();
        self.emit(Instruction::Load { result, slot, ty });
        Value::Local(result)
    }

    fn declare(&mut self, name: &Ident, slot: SlotId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.name.clone(), slot);
        }
    }

    fn lookup(&self, name: &str) -> Option<(SlotId, IrType)> {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                let ty = self
                    .function
                    .blocks
                    .iter()
                    .flat_map(|block| block.instructions.iter())
                    .find_map(|instruction| match instruction {
                        Instruction::Alloc { slot: found, ty } if *found == slot => {
                            Some(ty.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or(IrType::Opaque);
                return Some((slot, ty));
            }
        }
        None
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn leave_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn lower_block(
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

    fn lower_stmt(
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

    fn lower_variable(&mut self, decl: &nava_ast::VariableDecl, lowerer: &mut Lowerer) {
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

    fn lower_if(
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

    fn number_to_string(&mut self, value: Value) -> Value {
        let result = self.fresh_value();
        self.emit(Instruction::NumberToString { result, value });
        Value::Local(result)
    }

    fn lower_assign(&mut self, target: &Expr, value: &Value, lowerer: &mut Lowerer) {
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

    fn lower_expr(&mut self, expr: &Expr, lowerer: &mut Lowerer) -> Value {
        match &expr.kind {
            ExprKind::Number(value) => Value::Number(*value),
            ExprKind::Bool(value) => Value::Bool(*value),
            ExprKind::Null => Value::Null,
            ExprKind::String(value) => {
                let global = lowerer.string_global(value);
                Value::String(global)
            }
            ExprKind::Ident(name) => match self.lookup(name) {
                Some((slot, ty)) => self.load(slot, ty),
                None => {
                    lowerer.error(format!("unknown value '{name}' in IR"), expr.span);
                    Value::Number(0.0)
                }
            },
            ExprKind::Binary { op, left, right } => {
                let lhs_ty = lowerer.expr_ty(left);
                let rhs_ty = lowerer.expr_ty(right);
                let lhs = self.lower_expr(left, lowerer);
                let rhs = self.lower_expr(right, lowerer);
                let result = self.fresh_value();
                if lhs_ty == IrType::String || rhs_ty == IrType::String {
                    // Coerce a numeric operand to a string so both runtime
                    // calls receive `ptr` values (JavaScript-style `+`).
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
                    match op {
                        AstBinaryOp::Add => {
                            self.emit(Instruction::StringConcat {
                                result,
                                left: lhs,
                                right: rhs,
                            });
                        }
                        AstBinaryOp::Eq
                        | AstBinaryOp::StrictEq
                        | AstBinaryOp::NotEq
                        | AstBinaryOp::StrictNotEq => {
                            self.emit(Instruction::StringEquals {
                                result,
                                left: lhs,
                                right: rhs,
                            });
                            if matches!(op, AstBinaryOp::NotEq | AstBinaryOp::StrictNotEq) {
                                let negated = self.fresh_value();
                                self.emit(Instruction::Unary {
                                    result: negated,
                                    op: UnaryOp::Not,
                                    operand: Value::Local(result),
                                });
                                return Value::Local(negated);
                            }
                        }
                        _ => {
                            lowerer.error("unsupported string operator", expr.span);
                        }
                    }
                } else {
                    self.emit(Instruction::Binary {
                        result,
                        op: binary_op(*op),
                        lhs,
                        rhs,
                    });
                }
                Value::Local(result)
            }
            ExprKind::Unary { op, operand } => {
                let operand = self.lower_expr(operand, lowerer);
                match op {
                    AstUnaryOp::Pos => operand,
                    AstUnaryOp::Neg | AstUnaryOp::Not => {
                        let result = self.fresh_value();
                        self.emit(Instruction::Unary {
                            result,
                            op: match op {
                                AstUnaryOp::Neg => UnaryOp::Neg,
                                _ => UnaryOp::Not,
                            },
                            operand,
                        });
                        Value::Local(result)
                    }
                }
            }
            ExprKind::Assign { target, value } => {
                let value = self.lower_expr(value, lowerer);
                self.lower_assign(target, &value, lowerer);
                value
            }
            ExprKind::Call { callee, args } => {
                let callee_name = match &callee.kind {
                    ExprKind::Ident(name) => name.clone(),
                    ExprKind::Member { object, property } => match &object.kind {
                        ExprKind::Ident(object) => format!("{object}.{property}"),
                        _ => {
                            lowerer.error("unsupported call target in IR", callee.span);
                            "unknown".to_owned()
                        }
                    },
                    _ => {
                        lowerer.error("unsupported call target in IR", callee.span);
                        "unknown".to_owned()
                    }
                };
                let args: Vec<Value> = args
                    .iter()
                    .map(|arg| self.lower_expr(arg, lowerer))
                    .collect();
                if (callee_name.contains('.') || callee_name == "unknown")
                    && !Lowerer::is_known_builtin(&callee_name)
                {
                    lowerer.error(
                        format!("unsupported call target '{callee_name}' in IR"),
                        callee.span,
                    );
                }
                let result_ty = if Lowerer::is_known_builtin(&callee_name) {
                    IrType::Void
                } else {
                    lowerer.return_type_of(&callee_name)
                };
                let result = match result_ty {
                    IrType::Void => None,
                    _ => Some(self.fresh_value()),
                };
                self.emit(Instruction::Call {
                    result,
                    result_ty,
                    callee: callee_name,
                    args,
                });
                result.map_or(Value::Null, Value::Local)
            }
            ExprKind::Index { object, index } => {
                let object_ty = lowerer.expr_ty(object);
                let object = self.lower_expr(object, lowerer);
                let index = self.lower_expr(index, lowerer);
                let result = self.fresh_value();
                match object_ty {
                    IrType::Array(element_ty) => {
                        self.emit(Instruction::ArrayGet {
                            result,
                            array: object,
                            index,
                            element_ty: *element_ty,
                        });
                    }
                    IrType::String => {
                        self.emit(Instruction::StringCharAt {
                            result,
                            value: object,
                            index,
                        });
                    }
                    _ => {
                        lowerer.error("indexing requires an array or string", expr.span);
                    }
                }
                Value::Local(result)
            }
            ExprKind::Member { object, property } => {
                let object_ty = lowerer.expr_ty(object);
                let object = self.lower_expr(object, lowerer);
                let result = self.fresh_value();
                match object_ty {
                    IrType::String if property == "length" => {
                        self.emit(Instruction::StringLength {
                            result,
                            value: object,
                        });
                    }
                    IrType::Array(_) if property == "length" => {
                        self.emit(Instruction::ArrayLength {
                            result,
                            array: object,
                        });
                    }
                    IrType::Object(fields) => {
                        if let Some((field_index, (_, field_ty))) = fields
                            .iter()
                            .enumerate()
                            .find(|(_, (name, _))| name == property)
                        {
                            self.emit(Instruction::ObjectGet {
                                result,
                                object,
                                field_index: field_index as u32,
                                field_ty: field_ty.clone(),
                            });
                        } else {
                            lowerer.error(format!("unknown field '{property}'"), expr.span);
                        }
                    }
                    _ => lowerer.error("member access requires an object", expr.span),
                }
                Value::Local(result)
            }
            ExprKind::ArrayLit(items) => {
                let element_ty = match lowerer.expr_ty(expr) {
                    IrType::Array(inner) => *inner,
                    _ => IrType::Opaque,
                };
                let array = self.fresh_value();
                self.emit(Instruction::ArrayNew {
                    result: array,
                    element_ty: element_ty.clone(),
                    length: items.len() as u32,
                });
                for (index, item) in items.iter().enumerate() {
                    let value = self.lower_expr(item, lowerer);
                    self.emit(Instruction::ArraySet {
                        array: Value::Local(array),
                        index: Value::Number(index as f64),
                        value,
                        element_ty: element_ty.clone(),
                    });
                }
                Value::Local(array)
            }
            ExprKind::ObjectLit(fields) => {
                let field_types = match lowerer.expr_ty(expr) {
                    IrType::Object(fields) => fields,
                    _ => Vec::new(),
                };
                let object = self.fresh_value();
                self.emit(Instruction::ObjectNew {
                    result: object,
                    fields: field_types.clone(),
                });
                // Field order comes from the inferred object type (a sorted
                // map), not from source order; find each field's real index.
                for field in fields {
                    let value = self.lower_expr(&field.value, lowerer);
                    if let Some((index, (_, field_ty))) = field_types
                        .iter()
                        .enumerate()
                        .find(|(_, (name, _))| name == &field.name)
                    {
                        self.emit(Instruction::ObjectSet {
                            object: Value::Local(object),
                            field_index: index as u32,
                            value,
                            field_ty: field_ty.clone(),
                        });
                    } else {
                        lowerer.error(format!("unknown field '{}'", field.name), field.span);
                    }
                }
                Value::Local(object)
            }
            ExprKind::Paren(inner) => self.lower_expr(inner, lowerer),
        }
    }
}

fn binary_op(op: AstBinaryOp) -> BinaryOp {
    match op {
        AstBinaryOp::Add => BinaryOp::Add,
        AstBinaryOp::Sub => BinaryOp::Sub,
        AstBinaryOp::Mul => BinaryOp::Mul,
        AstBinaryOp::Div => BinaryOp::Div,
        AstBinaryOp::Mod => BinaryOp::Mod,
        AstBinaryOp::Eq => BinaryOp::Eq,
        AstBinaryOp::StrictEq => BinaryOp::Eq,
        AstBinaryOp::NotEq => BinaryOp::Ne,
        AstBinaryOp::StrictNotEq => BinaryOp::Ne,
        AstBinaryOp::Lt => BinaryOp::Lt,
        AstBinaryOp::Le => BinaryOp::Le,
        AstBinaryOp::Gt => BinaryOp::Gt,
        AstBinaryOp::Ge => BinaryOp::Ge,
        AstBinaryOp::And => BinaryOp::And,
        AstBinaryOp::Or => BinaryOp::Or,
    }
}
