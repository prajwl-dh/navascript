//! Tests for the shared diagnostics leaf.

use navascript_ast::{FileId, Span};
use navascript_diagnostics::{Diagnostic, Diagnostics, LineIndex};

#[test]
fn positions_are_one_based() {
    let lines = LineIndex::new("abc\ndef");
    assert_eq!(lines.position(0), (1, 1));
    assert_eq!(lines.position(5), (2, 2));
    assert_eq!(lines.line_text("abc\ndef", 2), "def");
}

#[test]
fn deduplicates_and_orders_by_span() {
    let span = Span::new(FileId(0), 1, 2);
    let lines = LineIndex::new("abcdef");
    let error = Diagnostic::error("same", span, &lines);
    let later = Diagnostic::error("later", Span::new(FileId(0), 3, 4), &lines);
    let diagnostics = Diagnostics::new(vec![later.clone(), error.clone(), error]);
    let messages: Vec<&str> = diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(messages, vec!["same", "later"]);
}
