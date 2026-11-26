//! NavaScript IR — the platform-independent middle of the compiler.
//!
//! The external seam is [`lower`]. It consumes the typed AST and produces an
//! owned [`IrModule`] plus diagnostics in the same shape as earlier phases.
//! [`verify`] and [`print`] are separate, deliberate entry points for the
//! verifier and textual debug output required by Phase 3.

mod access;
mod bindings;
mod builder;
mod calls;
mod checks;
mod classes;
mod closure_names;
mod closures;
mod exceptions;
mod expressions;
mod for_of;
mod generics;
mod ir;
mod loops;
mod lower;
mod operators;
mod print;
mod promises;
mod results;
mod statements;
mod values;
mod verify;
mod verify_calls;

pub use ir::{
    BasicBlock, BinaryOp, BlockId, GlobalId, Instruction, IrFunction, IrGlobal, IrModule, IrParam,
    IrType, SlotId, Terminator, UnaryOp, Value, ValueId,
};

/// IR name of the synthetic function holding top-level statements. It uses the
/// reserved prefix from `navascript-ast`, so user code cannot define it.
pub const ENTRY_FUNCTION: &str = "__navascript_entry";
pub use lower::{LowerOutput, lower};
pub use print::print;
pub use verify::{VerifyError, verify};
