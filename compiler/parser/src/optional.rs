//! Optional chaining postfix parsing (`?.prop`, `?.[index]`).

use nava_ast::{Expr, ExprKind, Span};
use nava_lexer::TokenKind;

use super::parser::Parser;

impl Parser<'_> {
    pub(super) fn parse_optional_postfix(&mut self, expr: &Expr) -> Option<Option<Expr>> {
        if self.kind() != TokenKind::QuestionDot {
            return Some(None);
        }
        self.bump();
        let value = if self.kind() == TokenKind::LBracket {
            self.bump();
            let index = self.parse_expr(0)?;
            let close = self.expect(TokenKind::RBracket, "expected ']'")?;
            Expr {
                kind: ExprKind::Index {
                    object: Box::new(expr.clone()),
                    index: Box::new(index),
                    optional: true,
                },
                span: Span::new(self.file, expr.span.start as usize, close.end as usize),
            }
        } else {
            let property = self.parse_ident("expected a property name")?;
            Expr {
                kind: ExprKind::Member {
                    object: Box::new(expr.clone()),
                    property: property.name,
                    optional: true,
                },
                span: Span::new(
                    self.file,
                    expr.span.start as usize,
                    property.span.end as usize,
                ),
            }
        };
        Some(Some(value))
    }
}
