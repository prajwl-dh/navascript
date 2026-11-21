//! Shared Nava diagnostics — a dependency-free leaf apart from `nava-ast`.
//!
//! Parser, type checker and driver all report through this one type, so the
//! diagnostics interface is the test surface for every front-end phase.

use std::fmt;

use nava_ast::Span;

/// Severity of a compiler diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => f.write_str("error"),
            Severity::Warning => f.write_str("warning"),
        }
    }
}

/// Maps byte offsets to 1-based line and column positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0u32];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push((index + 1) as u32);
            }
        }
        Self { starts }
    }

    pub fn position(&self, offset: u32) -> (u32, u32) {
        let line = self.starts.partition_point(|start| *start <= offset) as u32;
        let column = offset - self.starts[(line - 1) as usize] + 1;
        (line, column)
    }

    /// The text of one line, without its newline.
    pub fn line_text<'a>(&self, source: &'a str, line: u32) -> &'a str {
        let index = (line as usize).saturating_sub(1);
        let Some(&start) = self.starts.get(index) else {
            return "";
        };
        let start = start as usize;
        let end = source[start..]
            .find('\n')
            .map_or(source.len(), |offset| start + offset);
        &source[start..end]
    }
}

/// A diagnostic with a source position derived from its span.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub line: u32,
    pub column: u32,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        span: Span,
        lines: &LineIndex,
    ) -> Self {
        let (line, column) = lines.position(span.start);
        Self {
            severity,
            message: message.into(),
            span,
            line,
            column,
        }
    }

    pub fn error(message: impl Into<String>, span: Span, lines: &LineIndex) -> Self {
        Self::new(Severity::Error, message, span, lines)
    }
}

/// Deduplicated, source-ordered diagnostics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    /// Sort by span, then severity, then message, and remove exact duplicates.
    pub fn new(mut items: Vec<Diagnostic>) -> Self {
        items.sort_by(|a, b| {
            a.span
                .start
                .cmp(&b.span.start)
                .then(rank(a.severity).cmp(&rank(b.severity)))
                .then(a.message.cmp(&b.message))
        });
        items.dedup_by(|a, b| {
            a.span == b.span && a.severity == b.severity && a.message == b.message
        });
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }

    /// Render each diagnostic with a source snippet and caret.
    pub fn render(&self, source: &str) -> String {
        let lines = LineIndex::new(source);
        let mut out = String::new();
        for diagnostic in &self.items {
            out.push_str(&format!(
                "<input>:{}:{}: {}: {}\n",
                diagnostic.line, diagnostic.column, diagnostic.severity, diagnostic.message
            ));
            out.push_str("  ");
            out.push_str(lines.line_text(source, diagnostic.line));
            out.push('\n');
            out.push_str("  ");
            out.push_str(&" ".repeat(diagnostic.column.saturating_sub(1) as usize));
            let width = diagnostic.span.len().max(1) as usize;
            out.push_str(&"^".repeat(width));
            out.push('\n');
        }
        out
    }
}

fn rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nava_ast::FileId;

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
}
