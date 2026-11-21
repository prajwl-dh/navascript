//! Numeric literal lexing: radix prefixes, digits and separators.

use super::super::token::{TokenKind, radix_prefix};
use super::Scanner;

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

    pub(super) fn lex_number(&mut self) {
        let start = self.pos;
        if let Some((radix, _)) = radix_prefix(&self.source[self.pos..]) {
            self.advance_char();
            self.advance_char();
            self.lex_digits(radix);
            self.push(TokenKind::Number, start);
            return;
        }
        self.lex_digits(10);
        if self.peek() == Some('.')
            && self.source[self.pos + 1..]
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_digit())
        {
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
                while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    self.advance_char();
                }
            } else {
                self.pos = save;
            }
        }
        self.push(TokenKind::Number, start);
    }
}
