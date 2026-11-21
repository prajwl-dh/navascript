//! Nava lexer — a private internal seam of `nava-parser`.
//!
//! Converts source text into tokens carrying byte spans. It owns trivia
//! handling, keyword recognition, literals and operators so the parser never
//! sees raw characters.

/// The lexical class of a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Ident,
    Number,
    String,
    Let,
    Const,
    Var,
    Function,
    Return,
    If,
    Else,
    True,
    False,
    Null,
    Void,
    Type,
    Interface,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Arrow,
    EqEq,
    StrictEq,
    Bang,
    BangEq,
    StrictNotEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    Unknown,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub start: u32,
    pub end: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub errors: Vec<LexError>,
}

/// Tokenize UTF-8 source text. Spans are half-open byte offsets.
pub fn tokenize(source: &str) -> Lexed {
    Lexer::new(source).run()
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'a> Lexer<'a> {
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
            if ch.is_ascii_alphabetic() || ch == '_' || ch == '$' {
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
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
                self.advance_char();
            } else {
                break;
            }
        }
        let kind = keyword(&self.source[start..self.pos]);
        self.push(kind, start);
    }

    fn lex_number(&mut self) {
        let start = self.pos;
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.advance_char();
        }
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

    fn lex_string(&mut self) {
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
                self.advance_char();
                if self.peek().is_some() {
                    self.advance_char();
                }
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

    fn lex_operator_or_unknown(&mut self) {
        let start = self.pos;
        let operators: [(&str, TokenKind); 15] = [
            ("===", TokenKind::StrictEq),
            ("!==", TokenKind::StrictNotEq),
            ("==", TokenKind::EqEq),
            ("!=", TokenKind::BangEq),
            ("=>", TokenKind::Arrow),
            ("<=", TokenKind::Le),
            (">=", TokenKind::Ge),
            ("&&", TokenKind::AndAnd),
            ("||", TokenKind::OrOr),
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

fn keyword(word: &str) -> TokenKind {
    match word {
        "let" => TokenKind::Let,
        "const" => TokenKind::Const,
        "var" => TokenKind::Var,
        "function" => TokenKind::Function,
        "return" => TokenKind::Return,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        "void" => TokenKind::Void,
        "type" => TokenKind::Type,
        "interface" => TokenKind::Interface,
        _ => TokenKind::Ident,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source)
            .tokens
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn tokenizes_variable_declaration() {
        assert_eq!(
            kinds("let x = 42;"),
            vec![
                TokenKind::Let,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Number,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn recognizes_longest_operators_first() {
        assert_eq!(
            kinds("a===b !== c <= d => e"),
            vec![
                TokenKind::Ident,
                TokenKind::StrictEq,
                TokenKind::Ident,
                TokenKind::StrictNotEq,
                TokenKind::Ident,
                TokenKind::Le,
                TokenKind::Ident,
                TokenKind::Arrow,
                TokenKind::Ident,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_both_comment_styles() {
        let lexed = tokenize("// line\nlet /* block */ x = 1;");
        assert_eq!(
            lexed.tokens.into_iter().map(|t| t.kind).collect::<Vec<_>>(),
            vec![
                TokenKind::Let,
                TokenKind::Ident,
                TokenKind::Eq,
                TokenKind::Number,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
        assert!(lexed.errors.is_empty());
    }

    #[test]
    fn records_byte_spans() {
        let lexed = tokenize("a + b");
        let token = &lexed.tokens[1];
        assert_eq!((token.start, token.end), (2, 3));
        assert_eq!(token.lexeme, "+");
    }

    #[test]
    fn reports_unterminated_string() {
        let lexed = tokenize("\"oops");
        assert_eq!(lexed.errors.len(), 1);
        assert!(lexed.errors[0].message.contains("unterminated"));
    }

    #[test]
    fn handles_escapes_in_strings() {
        let lexed = tokenize("\"a\\nb\"");
        assert!(lexed.errors.is_empty());
        assert_eq!(lexed.tokens[0].kind, TokenKind::String);
    }

    #[test]
    fn reports_unknown_character() {
        let lexed = tokenize("let x = @;");
        assert_eq!(lexed.errors.len(), 1);
        assert_eq!(lexed.errors[0].message, "unexpected character");
    }
}
