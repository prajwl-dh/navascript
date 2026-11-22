//! Byte-level tokenization: trivia, literals, operators and EOF.

use super::token::{LexError, Lexed, Token, TokenKind, keyword};

mod number;
mod string;

/// Whether `ch` may continue an identifier (or follow a bigint suffix without
/// making the suffix part of a longer word).
pub(super) fn is_ident_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '$'
}

/// Tokenize UTF-8 source text. Spans are half-open byte offsets.
pub fn tokenize(source: &str) -> Lexed {
    Scanner::new(source).run()
}

struct Scanner<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn run(mut self) -> Lexed {
        while self.pos < self.source.len() {
            let ch = self.peek().expect("position is inside the source");
            if ch.is_whitespace() {
                self.advance_char();
                continue;
            }
            if self.starts_with("//") {
                self.skip_line_comment();
                continue;
            }
            if self.starts_with("/*") {
                self.skip_block_comment();
                continue;
            }
            if ch.is_alphabetic() || ch == '_' || ch == '$' {
                self.lex_ident_or_keyword();
                continue;
            }
            if ch.is_ascii_digit() {
                self.lex_number();
                continue;
            }
            if ch == '"' || ch == '\'' {
                self.lex_string();
                continue;
            }
            self.lex_operator_or_unknown();
        }

        let end = self.source.len() as u32;
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            start: end,
            end,
        });
        Lexed {
            tokens: self.tokens,
            errors: self.errors,
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn starts_with(&self, text: &str) -> bool {
        self.source[self.pos..].starts_with(text)
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            lexeme: self.source[start..self.pos].to_owned(),
            start: start as u32,
            end: self.pos as u32,
        });
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            self.advance_char();
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) {
        let start = self.pos;
        self.advance_char();
        self.advance_char();
        while self.pos < self.source.len() && !self.starts_with("*/") {
            self.advance_char();
        }
        if self.starts_with("*/") {
            self.advance_char();
            self.advance_char();
        } else {
            self.errors.push(LexError {
                start: start as u32,
                end: self.pos as u32,
                message: "unterminated block comment".to_owned(),
            });
        }
    }

    fn lex_ident_or_keyword(&mut self) {
        let start = self.pos;
        while self.peek().is_some_and(is_ident_continue) {
            self.advance_char();
        }
        let kind = keyword(&self.source[start..self.pos]);
        self.push(kind, start);
    }

    /// Consume digits in `radix`, allowing `_` only between two digits.
    fn lex_operator_or_unknown(&mut self) {
        let start = self.pos;
        let operators: [(&str, TokenKind); 24] = [
            ("?.", TokenKind::QuestionDot),
            ("===", TokenKind::StrictEq),
            ("!==", TokenKind::StrictNotEq),
            ("==", TokenKind::EqEq),
            ("!=", TokenKind::BangEq),
            ("=>", TokenKind::Arrow),
            ("<=", TokenKind::Le),
            (">=", TokenKind::Ge),
            ("&&", TokenKind::AndAnd),
            ("||", TokenKind::OrOr),
            ("**", TokenKind::StarStar),
            ("++", TokenKind::PlusPlus),
            ("--", TokenKind::MinusMinus),
            ("+=", TokenKind::PlusEq),
            ("-=", TokenKind::MinusEq),
            ("*=", TokenKind::StarEq),
            ("/=", TokenKind::SlashEq),
            ("%=", TokenKind::PercentEq),
            ("(", TokenKind::LParen),
            (")", TokenKind::RParen),
            ("{", TokenKind::LBrace),
            ("}", TokenKind::RBrace),
            ("[", TokenKind::LBracket),
            ("]", TokenKind::RBracket),
        ];
        if let Some((text, kind)) = operators
            .iter()
            .find(|(text, _)| self.starts_with(text))
            .map(|(text, kind)| (*text, *kind))
        {
            self.pos += text.len();
            self.push(kind, start);
            return;
        }

        let single = match self.peek() {
            Some(',') => Some(TokenKind::Comma),
            Some(';') => Some(TokenKind::Semicolon),
            Some(':') => Some(TokenKind::Colon),
            Some('.') => Some(TokenKind::Dot),
            Some('+') => Some(TokenKind::Plus),
            Some('-') => Some(TokenKind::Minus),
            Some('*') => Some(TokenKind::Star),
            Some('/') => Some(TokenKind::Slash),
            Some('%') => Some(TokenKind::Percent),
            Some('=') => Some(TokenKind::Eq),
            Some('!') => Some(TokenKind::Bang),
            Some('?') => Some(TokenKind::Question),
            Some('<') => Some(TokenKind::Lt),
            Some('>') => Some(TokenKind::Gt),
            _ => None,
        };

        match single {
            Some(kind) => {
                self.advance_char();
                self.push(kind, start);
            }
            None => {
                self.advance_char();
                self.push(TokenKind::Unknown, start);
                self.errors.push(LexError {
                    start: start as u32,
                    end: self.pos as u32,
                    message: "unexpected character".to_owned(),
                });
            }
        }
    }
}
