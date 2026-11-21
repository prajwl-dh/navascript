//! Tests for the lexer's public `tokenize` interface.

use nava_lexer::{TokenKind, tokenize};

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
fn recognizes_loop_keywords() {
    assert_eq!(
        kinds("while do for break continue"),
        vec![
            TokenKind::While,
            TokenKind::Do,
            TokenKind::For,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn recognizes_new_operators() {
    assert_eq!(
        kinds("** ++ -- += -= *= /= %= ?"),
        vec![
            TokenKind::StarStar,
            TokenKind::PlusPlus,
            TokenKind::MinusMinus,
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq,
            TokenKind::Question,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_based_literals_separators_and_unicode_identifiers() {
    let lexed = tokenize("let café = 0xFF + 0b1010 + 0o17 + 1_000;");
    assert!(lexed.errors.is_empty());
    let kinds: Vec<TokenKind> = lexed.tokens.into_iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Let,
            TokenKind::Ident,
            TokenKind::Eq,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::Plus,
            TokenKind::Number,
            TokenKind::Semicolon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_unicode_and_hex_escapes_in_strings() {
    let lexed = tokenize("\"\\x41\\u{42}\\u0043\"");
    assert!(lexed.errors.is_empty());
    assert_eq!(lexed.tokens[0].kind, TokenKind::String);
}

#[test]
fn recognizes_optional_chaining() {
    assert_eq!(
        kinds("a?.b?.[0]"),
        vec![
            TokenKind::Ident,
            TokenKind::QuestionDot,
            TokenKind::Ident,
            TokenKind::QuestionDot,
            TokenKind::LBracket,
            TokenKind::Number,
            TokenKind::RBracket,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn reports_invalid_hex_and_unicode_escapes() {
    let lexed = tokenize("\"\\xGG\\uZZZZ\"");
    assert!(
        lexed
            .errors
            .iter()
            .any(|error| error.message.contains("invalid \\x escape"))
    );
    assert!(
        lexed
            .errors
            .iter()
            .any(|error| error.message.contains("invalid \\u escape"))
    );
}

#[test]
fn reports_unknown_character() {
    let lexed = tokenize("let x = @;");
    assert_eq!(lexed.errors.len(), 1);
    assert_eq!(lexed.errors[0].message, "unexpected character");
}
