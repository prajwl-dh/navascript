//! Tests for the AST leaf's span type.

use nava_ast::{FileId, Span};

#[test]
fn span_length_and_emptiness() {
    let span = Span::new(FileId(0), 3, 7);
    assert_eq!(span.len(), 4);
    assert!(!span.is_empty());
    assert!(Span::new(FileId(0), 2, 2).is_empty());
}
