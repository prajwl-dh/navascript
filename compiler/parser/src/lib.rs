//! Nava parser — the deep front-end module.
//!
//! Its external seam is [`parse`]: source text in, `ParseOutput` out. The
//! implementation is split into private modules for parsing state, items,
//! type syntax, statements and expressions; only this module is public.

mod arrows;
mod expressions;
mod items;
mod operators;
mod parser;
mod statements;
mod types;

pub use nava_diagnostics::{Diagnostic, Diagnostics, Severity};

use nava_ast::{FileId, Program, Span};
use nava_diagnostics::LineIndex;
use nava_lexer::tokenize;
use parser::Parser;

/// The result of parsing: an owned program plus all diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutput {
    /// Best-effort program. It is partial when diagnostics contain errors.
    pub program: Program,
    pub diagnostics: Diagnostics,
}

/// Parse source text into a program and diagnostics.
///
/// Infallible and I/O-free: syntax errors become diagnostics, never `Err` or a
/// panic. The returned data is owned, so it outlives `source` and ports to Nava.
pub fn parse(source: &str) -> ParseOutput {
    let lexed = tokenize(source);
    let file = FileId(0);
    let lines = LineIndex::new(source);
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        file,
        lines,
        diagnostics: Vec::new(),
        depth: 0,
    };

    for error in lexed.errors {
        let span = Span::new(file, error.start as usize, error.end as usize);
        parser
            .diagnostics
            .push(Diagnostic::error(error.message, span, &parser.lines));
    }

    let program = parser.parse_program();
    ParseOutput {
        program,
        diagnostics: Diagnostics::new(parser.diagnostics),
    }
}
