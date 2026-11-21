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
    While,
    Do,
    For,
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
    StarStar,
    PlusPlus,
    MinusMinus,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Question,
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

/// Recognise a numeric base prefix and return `(radix, remaining text)`.
/// Shared by the scanner and the parser so prefix knowledge lives in one place.
pub fn radix_prefix(text: &str) -> Option<(u32, &str)> {
    const PREFIXES: [(&str, u32); 6] = [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
    ];
    PREFIXES
        .iter()
        .find_map(|(prefix, radix)| text.strip_prefix(prefix).map(|rest| (*radix, rest)))
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
        "while" => TokenKind::While,
        "do" => TokenKind::Do,
        "for" => TokenKind::For,
        _ => TokenKind::Ident,
    }
}
