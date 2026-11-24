//! Tests for the lexer's public `tokenize` interface.

use navascript_lexer::{TokenKind, tokenize};

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

#[test]
fn lexes_bigint_literals_with_prefixes_and_separators() {
    let lexed = tokenize("123n 0xffn 1_000n 0b10n 0o17n 42");
    assert!(lexed.errors.is_empty());
    let kinds: Vec<TokenKind> = lexed.tokens.iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::BigInt,
            TokenKind::BigInt,
            TokenKind::BigInt,
            TokenKind::BigInt,
            TokenKind::BigInt,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
    assert_eq!(lexed.tokens[0].lexeme, "123n");
    assert_eq!(lexed.tokens[1].lexeme, "0xffn");
}

#[test]
fn reports_bigint_suffix_on_a_fractional_literal() {
    let lexed = tokenize("1.5n");
    assert!(
        lexed
            .errors
            .iter()
            .any(|error| error.message.contains("bigint literals must be integers"))
    );
    assert_eq!(lexed.tokens[0].kind, TokenKind::Number);
}

#[test]
fn does_not_treat_a_bigint_suffix_as_part_of_a_longer_identifier() {
    let lexed = tokenize("123nfoo");
    assert_eq!(lexed.tokens[0].kind, TokenKind::Number);
    assert_eq!(lexed.tokens[1].kind, TokenKind::Ident);
    assert_eq!(lexed.tokens[1].lexeme, "nfoo");
}

#[test]
fn lexes_module_keywords() {
    assert_eq!(
        kinds("import { a } from \"./m.ns\"; export function f() {}"),
        vec![
            TokenKind::Import,
            TokenKind::LBrace,
            TokenKind::Ident,
            TokenKind::RBrace,
            TokenKind::Ident,
            TokenKind::String,
            TokenKind::Semicolon,
            TokenKind::Export,
            TokenKind::Function,
            TokenKind::Ident,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_union_and_intersection_type_operators() {
    assert_eq!(
        kinds("string | null & X"),
        vec![
            TokenKind::Ident,
            TokenKind::Pipe,
            TokenKind::Null,
            TokenKind::Amp,
            TokenKind::Ident,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_class_keywords() {
    assert_eq!(
        kinds("class A extends B { new this super }"),
        vec![
            TokenKind::Class,
            TokenKind::Ident,
            TokenKind::Extends,
            TokenKind::Ident,
            TokenKind::LBrace,
            TokenKind::New,
            TokenKind::This,
            TokenKind::Super,
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn recognizes_exception_keywords() {
    assert_eq!(
        kinds("throw try catch finally"),
        vec![
            TokenKind::Throw,
            TokenKind::Try,
            TokenKind::Catch,
            TokenKind::Finally,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn recognizes_async_keywords() {
    assert_eq!(
        kinds("async await"),
        vec![TokenKind::Async, TokenKind::Await, TokenKind::Eof]
    );
}

#[test]
fn lexes_optional_calls() {
    assert_eq!(
        kinds("f?.()"),
        vec![
            TokenKind::Ident,
            TokenKind::QuestionDot,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}
