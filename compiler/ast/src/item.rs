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
    Class(ClassDecl),
    /// `import { name, ... } from "path";`
    Import {
        names: Vec<Ident>,
        source: String,
        source_span: Span,
    },
    /// `export <item>` — the wrapped item is checked and lowered normally.
    Export(Box<Item>),
    TypeAlias(TypeAliasDecl),
    Interface(InterfaceDecl),
    Expr(Expr),
    /// A statement at program scope (e.g. a loop or a block), lowered into the
    /// synthetic `main` function alongside expression items.
    Stmt(Stmt),
}

impl ItemKind {
    /// Look through `export` wrappers so declaration passes can treat an
    /// exported item exactly like a local one.
    pub fn unwrap_export(&self) -> &ItemKind {
        match self {
            ItemKind::Export(inner) => inner.kind.unwrap_export(),
            other => other,
        }
    }

    /// The binding introduced by a declaration item, if any.
    pub fn declared_name(&self) -> Option<&str> {
        match self.unwrap_export() {
            ItemKind::Variable(decl) => Some(&decl.name.name),
            ItemKind::Function(decl) => Some(&decl.name.name),
            ItemKind::Class(decl) => Some(&decl.name.name),
            ItemKind::TypeAlias(decl) => Some(&decl.name.name),
            ItemKind::Interface(decl) => Some(&decl.name.name),
            _ => None,
        }
    }
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
    /// `async function` returns its value wrapped in a promise (ADR-0029).
    pub is_async: bool,
    pub type_params: Vec<Ident>,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeAnnotation>,
    pub body: Block,
}

/// A class declaration. Fields and methods are inherited from `base` at type
/// check time; methods lower to named functions with `this` as first parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub name: Ident,
    pub base: Option<Ident>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field {
        name: Ident,
        ty: Option<TypeAnnotation>,
    },
    Constructor {
        params: Vec<Param>,
        body: Block,
    },
    Method {
        name: Ident,
        params: Vec<Param>,
        return_ty: Option<TypeAnnotation>,
        body: Block,
    },
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
    /// `throw expr;` — only valid inside a `try` in the same function (ADR-0028).
    Throw(Expr),
    /// `try { ... } catch (e: T) { ... } finally { ... }` (ADR-0028).
    Try(TryStmt),
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

/// A `try` statement with an annotated catch binding and an optional `finally`.
#[derive(Debug, Clone, PartialEq)]
pub struct TryStmt {
    pub body: Block,
    pub catch_param: Param,
    pub catch_body: Block,
    pub finally_body: Option<Block>,
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
