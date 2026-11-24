//! NavaScript LLVM backend.
//!
//! Translation is deliberately textual: NavaScript IR → LLVM IR (`.ll`) → `llc` →
//! object → `clang` link. No LLVM Rust bindings are used (ADR-0004). The only
//! dependency is the IR crate, so the front end stays backend-independent.

mod instructions;
mod module;
mod runtime;
mod types;

pub use runtime::RUNTIME_FILES;

use navascript_ir::IrModule;

#[derive(Debug, Clone, PartialEq)]
pub struct CodegenError {
    pub message: String,
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Generate textual LLVM IR for a verified NavaScript module.
pub fn generate(module: &IrModule, target_triple: &str) -> Result<String, CodegenError> {
    module::emit_module(module, target_triple)
}
