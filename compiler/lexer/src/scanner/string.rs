//! String literal lexing and escape validation.

use super::super::token::{LexError, TokenKind};
use super::Scanner;

impl Scanner<'_> {
    pub(super) fn lex_string(&mut self) {
        let start = self.pos;
        let quote = self.advance_char().expect("opening quote");
        let mut closed = false;
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance_char();
                closed = true;
                break;
            }
            if ch == '\\' {
                let escape_start = self.pos;
                self.advance_char();
                self.consume_escape(escape_start);
            } else {
                self.advance_char();
            }
        }
        self.push(TokenKind::String, start);
        if !closed {
            self.errors.push(LexError {
                start: start as u32,
                end: self.pos as u32,
                message: "unterminated string literal".to_owned(),
            });
        }
    }

    /// Consume the body of an escape so the closing quote is found after
    /// multi-character escapes like `\u{1F600}` or `\x41`.
    fn consume_escape(&mut self, start: usize) {
        match self.peek() {
            Some('u') => {
                self.advance_char();
                if self.peek() == Some('{') {
                    self.advance_char();
                    let mut digits = 0;
                    let mut closed = false;
                    while let Some(ch) = self.peek() {
                        self.advance_char();
                        if ch == '}' {
                            closed = true;
                            break;
                        }
                        if ch.is_ascii_hexdigit() {
                            digits += 1;
                        }
                    }
                    if !closed || digits == 0 {
                        self.escape_error(start, "invalid \\u escape");
                    }
                } else {
                    let digits = self.consume_hex_digits(4);
                    if digits != 4 {
                        self.escape_error(start, "invalid \\u escape");
                    }
                }
            }
            Some('x') => {
                self.advance_char();
                let digits = self.consume_hex_digits(2);
                if digits != 2 {
                    self.escape_error(start, "invalid \\x escape");
                }
            }
            Some(_) => {
                self.advance_char();
            }
            None => self.escape_error(start, "unterminated escape sequence"),
        }
    }

    fn consume_hex_digits(&mut self, count: usize) -> usize {
        let mut digits = 0;
        for _ in 0..count {
            if self.peek().is_some_and(|ch| ch.is_ascii_hexdigit()) {
                self.advance_char();
                digits += 1;
            } else {
                break;
            }
        }
        digits
    }

    fn escape_error(&mut self, start: usize, message: &str) {
        self.errors.push(LexError {
            start: start as u32,
            end: self.pos as u32,
            message: message.to_owned(),
        });
    }
}
