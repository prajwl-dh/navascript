//! Nava lexer — a private internal seam of `nava-parser`.
//!
//! Converts source text into tokens carrying byte spans. The public interface
//! is [`tokenize`]; token vocabulary and byte-level scanning are private
//! implementation modules.

mod scanner;
mod token;

pub use scanner::tokenize;
pub use token::{LexError, Lexed, Token, TokenKind};
