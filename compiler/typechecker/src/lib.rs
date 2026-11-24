//! NavaScript type checker — the Phase 2 external seam.
//!
//! `check(&Program, &str) -> CheckOutput` hides the symbol table, scope
//! handling, type inference and assignability rules. It depends only on the
//! AST, the shared diagnostics leaf and the symbol/type leaf; LLVM is not
//! involved.

mod arrows;
mod bigint;
mod checker;
mod class_check;
mod classes;
mod declarations;
mod exceptions;
mod expressions;
mod generics;
mod narrowing;
mod optional;
mod statements;
mod types;

pub use checker::{CheckOutput, TypedProgram, check};
pub use classes::ClassInfo;
pub use types::{AliasTable, is_assignable, resolve_type};
