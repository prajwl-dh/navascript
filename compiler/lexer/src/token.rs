//! Token vocabulary and lexer output.

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

pub(super) fn keyword(word: &str) -> TokenKind {
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
