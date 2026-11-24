//! Free-name analysis for closures (ADR-0027).
//!
//! One visitor answers two questions: which names an arrow captures, and which
//! bindings a function body must box because some arrow inside it captures
//! them. The visitor is deliberately conservative: a name declared *anywhere*
//! in a body counts as local. An over-captured field is harmless (the inner
//! declaration shadows it), while a missing capture would miscompile.

use std::collections::{BTreeSet, HashSet};

use navascript_ast::{ArrowBody, Block, Expr, ExprKind, Item, ItemKind, Param, Stmt, StmtKind};

/// Names referenced by an arrow body that the body itself does not bind.
pub(super) fn free_names(params: &[Param], body: &ArrowBody) -> Vec<String> {
    let mut collector = FreeNameCollector::with_params(params);
    collector.body(body);
    collector.free.into_iter().collect()
}

/// Names captured by any arrow appearing in an arrow body.
pub(super) fn captures_in_body(body: &ArrowBody) -> HashSet<String> {
    let mut collector = FreeNameCollector::default();
    collector.body(body);
    collector.captures
}

/// Names captured by any arrow appearing in an expression.
pub(super) fn captures_in_expr(expr: &Expr) -> HashSet<String> {
    let mut collector = FreeNameCollector::default();
    collector.expr(expr);
    collector.captures
}

/// Names captured by any arrow appearing in a statement.
pub(super) fn captures_in_stmt(stmt: &Stmt) -> HashSet<String> {
    let mut collector = FreeNameCollector::default();
    collector.statement(stmt);
    collector.captures
}

/// Names captured by any arrow appearing in a block.
pub(super) fn captures_in_block(block: &Block) -> HashSet<String> {
    let mut collector = FreeNameCollector::default();
    collector.block(block);
    collector.captures
}

/// Names captured by arrows in program-level items (used for synthetic `main`).
pub(super) fn captures_in_items(items: &[&Item]) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in items {
        match item.kind.unwrap_export() {
            ItemKind::Variable(decl) => {
                if let Some(init) = &decl.init {
                    names.extend(captures_in_expr(init));
                }
            }
            ItemKind::Expr(expr) => names.extend(captures_in_expr(expr)),
            ItemKind::Stmt(stmt) => names.extend(captures_in_stmt(stmt)),
            _ => {}
        }
    }
    names
}

#[derive(Default)]
struct FreeNameCollector {
    bound: HashSet<String>,
    free: BTreeSet<String>,
    captures: HashSet<String>,
}

impl FreeNameCollector {
    fn with_params(params: &[Param]) -> Self {
        Self {
            bound: params.iter().map(|param| param.name.name.clone()).collect(),
            free: BTreeSet::new(),
            captures: HashSet::new(),
        }
    }

    fn body(&mut self, body: &ArrowBody) {
        match body {
            ArrowBody::Expr(expr) => self.expr(expr),
            ArrowBody::Block(block) => self.block(block),
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Ident(name) => {
                if !self.bound.contains(name) {
                    self.free.insert(name.clone());
                }
            }
            ExprKind::This => {
                if !self.bound.contains("this") {
                    self.free.insert("this".to_owned());
                }
            }
            ExprKind::Arrow { params, body, .. } => {
                self.captures.extend(free_names(params, body));
                let outer = self.bound.clone();
                for param in params {
                    self.bound.insert(param.name.name.clone());
                }
                self.body(body);
                self.bound = outer;
            }
            ExprKind::Binary { left, right, .. }
            | ExprKind::Assign {
                target: left,
                value: right,
            }
            | ExprKind::CompoundAssign {
                target: left,
                value: right,
                ..
            }
            | ExprKind::Index {
                object: left,
                index: right,
                ..
            } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Await(operand) => self.expr(operand),
            ExprKind::Unary { operand, .. }
            | ExprKind::Update { operand, .. }
            | ExprKind::Member {
                object: operand, ..
            }
            | ExprKind::Paren(operand) => self.expr(operand),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.expr(condition);
                self.expr(then_expr);
                self.expr(else_expr);
            }
            ExprKind::Call { callee, args, .. } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::New { args, .. }
            | ExprKind::SuperCall { args }
            | ExprKind::ArrayLit(args) => {
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::ObjectLit(fields) => {
                for field in fields {
                    self.expr(&field.value);
                }
            }
            ExprKind::Number(_)
            | ExprKind::BigInt(_)
            | ExprKind::String(_)
            | ExprKind::Bool(_)
            | ExprKind::Null
            | ExprKind::Super => {}
        }
    }

    fn block(&mut self, block: &Block) {
        let outer = self.bound.clone();
        for statement in &block.statements {
            self.statement(statement);
        }
        self.bound = outer;
    }

    fn statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Expr(expr) => self.expr(expr),
            StmtKind::Variable(decl) => {
                if let Some(init) = &decl.init {
                    self.expr(init);
                }
                self.bound.insert(decl.name.name.clone());
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::If(if_stmt) => {
                self.expr(&if_stmt.condition);
                self.block(&if_stmt.then_branch);
                match &if_stmt.else_branch {
                    Some(navascript_ast::ElseBranch::Block(block)) => self.block(block),
                    Some(navascript_ast::ElseBranch::If(nested)) => {
                        let outer = self.bound.clone();
                        self.expr(&nested.condition);
                        self.block(&nested.then_branch);
                        self.bound = outer;
                    }
                    None => {}
                }
            }
            StmtKind::While(stmt) | StmtKind::DoWhile(stmt) => {
                self.expr(&stmt.condition);
                self.block(&stmt.body);
            }
            StmtKind::For(stmt) => {
                let outer = self.bound.clone();
                if let Some(init) = &stmt.init {
                    self.statement(init);
                }
                if let Some(condition) = &stmt.condition {
                    self.expr(condition);
                }
                if let Some(update) = &stmt.update {
                    self.expr(update);
                }
                self.block(&stmt.body);
                self.bound = outer;
            }
            StmtKind::ForOf(stmt) => {
                self.expr(&stmt.iterable);
                let outer = self.bound.clone();
                self.bound.insert(stmt.binding.name.clone());
                self.block(&stmt.body);
                self.bound = outer;
            }
            StmtKind::Throw(expr) => self.expr(expr),
            StmtKind::Try(try_stmt) => {
                self.block(&try_stmt.body);
                self.block(&try_stmt.catch_body);
                if let Some(finally) = &try_stmt.finally_body {
                    self.block(finally);
                }
            }
            StmtKind::Block(block) => self.block(block),
            StmtKind::Break | StmtKind::Continue | StmtKind::Empty => {}
        }
    }
}
