//! Shared postfix call parsing (`f(args)` and `f?.(args)`).

use nava_ast::{Expr, ExprKind, Span};
use nava_lexer::TokenKind;

use super::parser::Parser;

impl Parser<'_> {
    /// Consume `(args)` following `callee`. The caller leaves the current token
    /// on the opening parenthesis; `optional` records which syntax was used.
    pub(super) fn parse_call_tail(&mut self, callee: Expr, optional: bool) -> Option<Expr> {
        self.bump();
        let mut args = Vec::new();
        while self.kind() != TokenKind::RParen && self.kind() != TokenKind::Eof {
            args.push(self.parse_expr(0)?);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        let close = self.expect(TokenKind::RParen, "expected ')' after arguments")?;
        let span = Span::new(self.file, callee.span.start as usize, close.end as usize);
        Some(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                args,
                optional,
            },
            span,
        })
    }
}
