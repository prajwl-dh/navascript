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
        kinds("while do for"),
        vec![
            TokenKind::While,
            TokenKind::Do,
            TokenKind::For,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn reports_unknown_character() {
    let lexed = tokenize("let x = @;");
    assert_eq!(lexed.errors.len(), 1);
    assert_eq!(lexed.errors[0].message, "unexpected character");
}
