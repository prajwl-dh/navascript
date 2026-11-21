//! Statements, declarations and the program root.

use super::expr::Expr;
use super::span::Span;
use super::types::{Ident, InterfaceDecl, TypeAliasDecl, TypeAnnotation};

/// A parsed source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Variable(VariableDecl),
    Function(FunctionDecl),
    TypeAlias(TypeAliasDecl),
    Interface(InterfaceDecl),
    Expr(Expr),
    /// A statement at program scope (e.g. a loop or a block), lowered into the
    /// synthetic `main` function alongside expression items.
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: Ident,
    pub ty: Option<TypeAnnotation>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeAnnotation>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    Variable(VariableDecl),
    Return(Option<Expr>),
    If(IfStmt),
    While(WhileStmt),
    DoWhile(WhileStmt),
    For(ForStmt),
    ForOf(ForOfStmt),
    Break,
    Continue,
    Block(Block),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Block,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    If(Box<IfStmt>),
    Block(Block),
}

/// Shared shape for `while (c) b` and `do b while (c)`.
#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: Block,
}

/// C-style `for (init; condition; update) body`. A missing condition means an
/// unconditional loop.
#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub init: Option<Box<Stmt>>,
    pub condition: Option<Expr>,
    pub update: Option<Expr>,
    pub body: Block,
}

/// `for (binding of iterable) body`. `of` is contextual, not a keyword.
#[derive(Debug, Clone, PartialEq)]
pub struct ForOfStmt {
    pub binding: Ident,
    pub iterable: Expr,
    pub body: Block,
    /// True for `let x of ...` (new binding), false for `x of ...` (assignment).
    pub decl: bool,
}
