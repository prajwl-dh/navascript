//! Numeric literal lexing: radix prefixes, separators and bigint suffixes.

use super::super::token::{LexError, TokenKind, radix_prefix};
use super::{Scanner, is_ident_continue};

impl Scanner<'_> {
    /// Consume digits in `radix`, allowing `_` only between two digits.
    fn lex_digits(&mut self, radix: u32) {
        loop {
            let digit = self.peek().is_some_and(|ch| ch.is_digit(radix));
            let separator = self.peek() == Some('_')
                && self.source[self.pos + 1..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_digit(radix));
            if digit || separator {
                self.advance_char();
            } else {
                break;
            }
        }
    }

    /// A `n` suffix only terminates a literal when it is not itself the start
    /// of a longer identifier (`123nfoo` is not a bigint literal).
    fn bigint_suffix_valid(&self) -> bool {
        !self.source[self.pos + 1..]
            .chars()
            .next()
            .is_some_and(is_ident_continue)
    }

    /// Emit the token for a literal that ended at `start`, recognising a valid
    /// bigint suffix and diagnosing `n` attached to a fractional/exponent form.
    fn push_number(&mut self, start: usize, integer: bool) {
        let mut bigint = false;
        if self.peek() == Some('n') {
            if integer && self.bigint_suffix_valid() {
                bigint = true;
                self.advance_char();
            } else if self.bigint_suffix_valid() {
                self.advance_char();
                self.errors.push(LexError {
                    start: self.pos as u32 - 1,
                    end: self.pos as u32,
                    message: "bigint literals must be integers".to_owned(),
                });
            }
        }
        self.push(
            if bigint {
                TokenKind::BigInt
            } else {
                TokenKind::Number
            },
            start,
        );
    }

    pub(super) fn lex_number(&mut self) {
        let start = self.pos;
        if let Some((radix, _)) = radix_prefix(&self.source[self.pos..]) {
            self.advance_char();
            self.advance_char();
            self.lex_digits(radix);
            self.push_number(start, true);
            return;
        }
        self.lex_digits(10);
        let mut integer = true;
        if self.peek() == Some('.')
            && self.source[self.pos + 1..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
        {
            integer = false;
            self.advance_char();
            while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                self.advance_char();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let save = self.pos;
            self.advance_char();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance_char();
            }
            if self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                integer = false;
                while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    self.advance_char();
                }
            } else {
                self.pos = save;
            }
        }
        self.push_number(start, integer);
    }
}
