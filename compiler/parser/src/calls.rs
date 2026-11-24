//! Shared postfix call parsing (`f(args)`, `f<T>(args)` and `f?.(args)`).

use navascript_ast::{Expr, ExprKind, Span, TypeExpr};
use navascript_lexer::TokenKind;

use super::parser::Parser;

impl Parser<'_> {
    /// A `<` is a type-argument list only when a matching `>` is immediately
    /// followed by `(`. Otherwise it stays the ordinary less-than operator.
    pub(super) fn type_args_ahead(&self) -> bool {
        if self.kind() != TokenKind::Lt {
            return false;
        }
        let mut depth = 0_i32;
        for (offset, token) in self.tokens[self.pos..].iter().enumerate() {
            match token.kind {
                TokenKind::Lt => depth += 1,
                TokenKind::Gt => {
                    depth -= 1;
                    if depth == 0 {
                        return self
                            .tokens
                            .get(self.pos + offset + 1)
                            .is_some_and(|next| next.kind == TokenKind::LParen);
                    }
                }
                TokenKind::Semicolon | TokenKind::LBrace | TokenKind::Eof => return false,
                _ => {}
            }
        }
        false
    }

    fn parse_call_type_args(&mut self) -> Option<Vec<TypeExpr>> {
        self.parse_angle_args()
    }

    /// Consume `<T>?(args)` following `callee`. The caller leaves the current
    /// token on either `<` or `(`; `optional` records which syntax was used.
    pub(super) fn parse_call(&mut self, callee: Expr, optional: bool) -> Option<Expr> {
        let type_args = if self.type_args_ahead() {
            self.parse_call_type_args()?
        } else {
            Vec::new()
        };
        self.expect(TokenKind::LParen, "expected '(' after callee")?;
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
                type_args,
                optional,
            },
            span,
        })
    }
}
