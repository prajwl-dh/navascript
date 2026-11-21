//! Nava IR — the platform-independent middle of the compiler.
//!
//! The external seam is [`lower`]. It consumes the typed AST and produces an
//! owned [`IrModule`] plus diagnostics in the same shape as earlier phases.
//! [`verify`] and [`print`] are separate, deliberate entry points for the
//! verifier and textual debug output required by Phase 3.

mod builder;
mod checks;
mod expressions;
mod ir;
mod loops;
mod lower;
mod print;
mod statements;
mod verify;

pub use ir::{
    BasicBlock, BinaryOp, BlockId, GlobalId, Instruction, IrFunction, IrGlobal, IrModule, IrParam,
    IrType, SlotId, Terminator, UnaryOp, Value, ValueId,
};
pub use lower::{LowerOutput, lower};
pub use print::print;
pub use verify::{VerifyError, verify};
