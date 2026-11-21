//! Arrow function expression parsing.

use nava_ast::{ArrowBody, Expr, ExprKind, Param, Span};
use nava_lexer::{Token, TokenKind};

use super::parser::Parser;

impl Parser<'_> {
    /// Look past a parenthesised parameter list (and optional return type) for
    /// `=>`, without consuming anything.
    pub(super) fn arrow_ahead(&self) -> bool {
        let mut depth = 0usize;
        let mut index = self.pos;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        let mut next = index + 1;
                        if self
                            .tokens
                            .get(next)
                            .is_some_and(|token| token.kind == TokenKind::Colon)
                        {
                            next += 1; // skip the optional return type annotation
                            while next < self.tokens.len()
                                && self.tokens[next].kind != TokenKind::Arrow
                                && self.tokens[next].kind != TokenKind::Eof
                            {
                                next += 1;
                            }
                        }
                        return self
                            .tokens
                            .get(next)
                            .is_some_and(|token| token.kind == TokenKind::Arrow);
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            index += 1;
        }
        false
    }

    /// Parse an identifier, or the `ident => body` shorthand.
    pub(super) fn parse_primary_ident(&mut self, token: &Token) -> Option<Expr> {
        if self
            .tokens
            .get(self.pos + 1)
            .is_some_and(|next| next.kind == TokenKind::Arrow)
        {
            return self.parse_ident_arrow(token);
        }
        self.bump();
        Some(Expr {
            kind: ExprKind::Ident(token.lexeme.clone()),
            span: Span::new(self.file, token.start as usize, token.end as usize),
        })
    }

    /// Parse `(params) [=> return type] => body`.
    pub(super) fn parse_paren_arrow(&mut self, start: usize) -> Option<Expr> {
        let params = self.parse_arrow_params()?;
        self.parse_arrow(params, start)
    }

    /// Parse the `ident => body` shorthand.
    pub(super) fn parse_ident_arrow(&mut self, token: &Token) -> Option<Expr> {
        self.bump(); // identifier already checked by the caller
        let param = Param {
            name: nava_ast::Ident {
                name: token.lexeme.clone(),
                span: Span::new(self.file, token.start as usize, token.end as usize),
            },
            ty: None,
            span: Span::new(self.file, token.start as usize, token.end as usize),
        };
        self.parse_arrow(vec![param], token.start as usize)
    }

    /// Parse the parameter list after the opening `(`.
    pub(super) fn parse_arrow_params(&mut self) -> Option<Vec<Param>> {
        self.bump(); // `(`
        let mut params = Vec::new();
        while self.kind() != TokenKind::RParen && self.kind() != TokenKind::Eof {
            let start = self.current().start;
            let name = self.parse_ident("expected a parameter name")?;
            let ty = if self.eat(TokenKind::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let end = self.current().end;
            params.push(Param {
                name,
                ty,
                span: Span::new(self.file, start as usize, end as usize),
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected ')' after arrow parameters")?;
        Some(params)
    }

    pub(super) fn parse_arrow(&mut self, params: Vec<Param>, start: usize) -> Option<Expr> {
        let return_ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect(TokenKind::Arrow, "expected '=>' after arrow parameters")?;
        let body = if self.kind() == TokenKind::LBrace {
            ArrowBody::Block(self.parse_block()?)
        } else {
            ArrowBody::Expr(Box::new(self.parse_expr(0)?))
        };
        let end = self.previous_end(start);
        Some(Expr {
            kind: ExprKind::Arrow {
                params,
                return_ty,
                body,
            },
            span: Span::new(self.file, start, end),
        })
    }
}
