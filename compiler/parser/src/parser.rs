//! Parser state, driver loop and shared token helpers.

use navascript_ast::{FileId, Program, Span};
use navascript_diagnostics::{Diagnostic, LineIndex};
use navascript_lexer::{Token, TokenKind, radix_prefix};

/// Keeps `parse` infallible: deeply nested input is diagnosed, not overflowed.
pub(super) const MAX_DEPTH: u32 = 128;

pub(super) struct Parser<'a> {
    pub(super) source: &'a str,
    pub(super) tokens: Vec<Token>,
    pub(super) pos: usize,
    pub(super) file: FileId,
    pub(super) lines: LineIndex,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) depth: u32,
}

impl<'a> Parser<'a> {
    pub(super) fn parse_program(&mut self) -> Program {
        let items = self.parse_until(|kind| kind == TokenKind::Eof, Self::parse_item);
        Program {
            items,
            span: Span::new(self.file, 0, self.source.len()),
        }
    }

    /// Shared list loop: parse one node, recover on failure, always progress.
    pub(super) fn parse_until<T>(
        &mut self,
        done: impl Fn(TokenKind) -> bool,
        mut parse_one: impl FnMut(&mut Self) -> Option<T>,
    ) -> Vec<T> {
        let mut nodes = Vec::new();
        while !done(self.kind()) {
            let before = self.pos;
            if let Some(node) = parse_one(self) {
                nodes.push(node);
            } else {
                self.recover();
            }
            if self.pos == before {
                self.bump();
            }
        }
        nodes
    }

    pub(super) fn enter(&mut self) -> bool {
        if self.depth >= MAX_DEPTH {
            self.error("expression nesting is too deep", self.current_span());
            return false;
        }
        self.depth += 1;
        true
    }

    pub(super) fn leave(&mut self) {
        self.depth -= 1;
    }

    pub(super) fn expect(&mut self, kind: TokenKind, message: &str) -> Option<Token> {
        if self.kind() == kind {
            let token = self.current().clone();
            self.bump();
            Some(token)
        } else {
            self.error(message, self.current_span());
            None
        }
    }

    pub(super) fn expect_semicolon(&mut self) -> u32 {
        if self.kind() == TokenKind::Semicolon {
            let end = self.current().end;
            self.bump();
            end
        } else {
            self.error("expected ';'", self.current_span());
            self.current().start
        }
    }

    pub(super) fn eat(&mut self, kind: TokenKind) -> bool {
        if self.kind() == kind {
            self.bump();
            true
        } else {
            false
        }
    }

    pub(super) fn recover(&mut self) {
        while !matches!(
            self.kind(),
            TokenKind::Semicolon
                | TokenKind::RBrace
                | TokenKind::Eof
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::Var
                | TokenKind::Function
                | TokenKind::Return
                | TokenKind::If
                | TokenKind::Else
        ) {
            self.bump();
        }
        if self.kind() == TokenKind::Semicolon {
            self.bump();
        }
    }

    pub(super) fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub(super) fn kind(&self) -> TokenKind {
        self.current().kind
    }

    pub(super) fn current_span(&self) -> Span {
        token_span(self.current(), self.file)
    }

    pub(super) fn bump(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    pub(super) fn previous_end(&self, fallback: usize) -> usize {
        if self.pos > 0 {
            self.tokens[self.pos - 1].end as usize
        } else {
            fallback
        }
    }

    pub(super) fn error(&mut self, message: impl Into<String>, span: Span) {
        let diagnostic = Diagnostic::error(message, span, &self.lines);
        self.diagnostics.push(diagnostic);
    }
}

pub(super) fn token_span(token: &Token, file: FileId) -> Span {
    Span::new(file, token.start as usize, token.end as usize)
}

/// Remove `_` separators shared by number and bigint literal decoding.
fn strip_separators(lexeme: &str) -> String {
    lexeme.chars().filter(|&ch| ch != '_').collect()
}

/// Parse a bigint lexeme (`123n`, `0xffn`, `1_000n`). NavaScript stores bigints as
/// `i64`; out-of-range literals are diagnosed by the caller.
pub(super) fn decode_bigint(lexeme: &str) -> Option<i64> {
    let digits = lexeme.strip_suffix('n')?;
    let clean = strip_separators(digits);
    let (radix, digits) = radix_prefix(&clean).unwrap_or((10, clean.as_str()));
    i128::from_str_radix(digits, radix)
        .ok()
        .and_then(|value| i64::try_from(value).ok())
}

/// Parse a numeric lexeme, including `0x`/`0b`/`0o` prefixes and `_` separators.
pub(super) fn decode_number(lexeme: &str) -> f64 {
    let clean = strip_separators(lexeme);
    let (radix, digits) = radix_prefix(&clean).unwrap_or((10, clean.as_str()));
    u64::from_str_radix(digits, radix)
        .map(|value| value as f64)
        .unwrap_or_else(|_| clean.parse::<f64>().unwrap_or(f64::NAN))
}

pub(super) fn decode_string(lexeme: &str) -> String {
    if lexeme.len() < 2 {
        return String::new();
    }
    let inner = &lexeme[1..lexeme.len() - 1];
    let chars: Vec<char> = inner.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        index += 1;
        if ch != '\\' || index >= chars.len() {
            out.push(ch);
            continue;
        }
        let escape = chars[index];
        index += 1;
        match escape {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '0' => out.push('\0'),
            '\\' => out.push('\\'),
            '"' => out.push('"'),
            '\'' => out.push('\''),
            'x' => {
                let digits: String = chars[index..].iter().take(2).collect();
                if let Ok(value) = u32::from_str_radix(&digits, 16) {
                    if let Some(decoded) = char::from_u32(value) {
                        out.push(decoded);
                    }
                }
                index += digits.len();
            }
            'u' => {
                if chars.get(index) == Some(&'{') {
                    index += 1;
                    let start = index;
                    while index < chars.len() && chars[index] != '}' {
                        index += 1;
                    }
                    let digits: String = chars[start..index].iter().collect();
                    if index < chars.len() {
                        index += 1; // closing brace
                    }
                    if let Ok(value) = u32::from_str_radix(&digits, 16) {
                        if let Some(decoded) = char::from_u32(value) {
                            out.push(decoded);
                        }
                    }
                } else {
                    let digits: String = chars[index..].iter().take(4).collect();
                    if let Ok(value) = u32::from_str_radix(&digits, 16) {
                        if let Some(decoded) = char::from_u32(value) {
                            out.push(decoded);
                        }
                    }
                    index += digits.len();
                }
            }
            other => out.push(other),
        }
    }
    out
}
