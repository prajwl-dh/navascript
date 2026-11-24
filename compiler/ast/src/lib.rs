//! NavaScript abstract syntax tree — a dependency-free leaf crate.
//!
//! `Program`, `Span` and `FileId` are shared with the type checker and IR
//! lowering. The crate has one public surface (this module) and three private
//! implementation modules: span, expressions, types and declarations.

mod expr;
mod item;
mod span;
mod types;

pub use expr::{ArrowBody, AssignOp, BinaryOp, Expr, ExprKind, ObjectField, UnaryOp, UpdateOp};
pub use item::{
    Block, ClassDecl, ClassMember, ElseBranch, ForOfStmt, ForStmt, FunctionDecl, IfStmt, Item,
    ItemKind, Param, Program, Stmt, StmtKind, VariableDecl, WhileStmt,
};
pub use span::{FileId, Span};
pub use types::{Ident, InterfaceDecl, TypeAliasDecl, TypeAnnotation, TypeExpr, TypeField};
