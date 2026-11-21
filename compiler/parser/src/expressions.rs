//! Expression parsing: precedence, unary/postfix operators and literals.

use nava_ast::{BinaryOp, Expr, ExprKind, ObjectField, Span, UnaryOp};
use nava_lexer::TokenKind;

use super::parser::Parser;
use super::parser::{decode_number, decode_string};

impl<'a> Parser<'a> {
    pub(super) fn parse_expr(&mut self, min_prec: u8) -> Option<Expr> {
        if !self.enter() {
            return None;
        }
        let result = self.parse_expr_inner(min_prec);
        self.leave();
        result
    }

    pub(super) fn parse_expr_inner(&mut self, min_prec: u8) -> Option<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            if min_prec == 0 && self.kind() == TokenKind::Eq {
                self.bump();
                let value = self.parse_expr(0)?;
                let span = Span::new(self.file, left.span.start as usize, value.span.end as usize);
                left = Expr {
                    kind: ExprKind::Assign {
                        target: Box::new(left),
                        value: Box::new(value),
                    },
                    span,
                };
                continue;
            }
            if min_prec == 0 {
                if let Some(op) = Self::compound_assign_op(self.kind()) {
                    self.bump();
                    let value = self.parse_expr(0)?;
                    let span =
                        Span::new(self.file, left.span.start as usize, value.span.end as usize);
                    left = Expr {
                        kind: ExprKind::CompoundAssign {
                            op,
                            target: Box::new(left),
                            value: Box::new(value),
                        },
                        span,
                    };
                    continue;
                }
            }
            let Some((op, prec)) = self.peek_binary_op() else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.bump();
            // `**` is right-associative; other binary operators are left-associative.
            let right = self.parse_expr(if op == BinaryOp::Pow { prec } else { prec + 1 })?;
            let span = Span::new(self.file, left.span.start as usize, right.span.end as usize);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        if min_prec == 0 && self.kind() == TokenKind::Question {
            left = self.parse_ternary_tail(left)?;
        }
        Some(left)
    }

    pub(super) fn peek_binary_op(&self) -> Option<(BinaryOp, u8)> {
        let op = match self.kind() {
            TokenKind::OrOr => BinaryOp::Or,
            TokenKind::AndAnd => BinaryOp::And,
            TokenKind::EqEq => BinaryOp::Eq,
            TokenKind::StrictEq => BinaryOp::StrictEq,
            TokenKind::BangEq => BinaryOp::NotEq,
            TokenKind::StrictNotEq => BinaryOp::StrictNotEq,
            TokenKind::Lt => BinaryOp::Lt,
            TokenKind::Le => BinaryOp::Le,
            TokenKind::Gt => BinaryOp::Gt,
            TokenKind::Ge => BinaryOp::Ge,
            TokenKind::Plus => BinaryOp::Add,
            TokenKind::Minus => BinaryOp::Sub,
            TokenKind::Star => BinaryOp::Mul,
            TokenKind::Slash => BinaryOp::Div,
            TokenKind::Percent => BinaryOp::Mod,
            TokenKind::StarStar => BinaryOp::Pow,
            _ => return None,
        };
        let precedence = match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::StrictEq | BinaryOp::NotEq | BinaryOp::StrictNotEq => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 6,
            BinaryOp::Pow => 7,
        };
        Some((op, precedence))
    }

    pub(super) fn parse_unary(&mut self) -> Option<Expr> {
        if !self.enter() {
            return None;
        }
        let result = self.parse_unary_inner();
        self.leave();
        result
    }

    pub(super) fn parse_unary_inner(&mut self) -> Option<Expr> {
        if let Some(expr) = self.parse_prefix_update()? {
            return Some(expr);
        }
        let op = match self.kind() {
            TokenKind::Bang => UnaryOp::Not,
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Plus => UnaryOp::Pos,
            _ => return self.parse_postfix(),
        };
        let start = self.current().start;
        self.bump();
        let operand = self.parse_unary()?;
        let span = Span::new(self.file, start as usize, operand.span.end as usize);
        Some(Expr {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(operand),
            },
            span,
        })
    }

    pub(super) fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if let Some(updated) = self.parse_postfix_update(&expr)? {
                expr = updated;
                continue;
            }
            if let Some(optional) = self.parse_optional_postfix(&expr)? {
                expr = optional;
                continue;
            }
            match self.kind() {
                TokenKind::Dot => {
                    self.bump();
                    let property = self.parse_ident("expected a property name")?;
                    let span = Span::new(
                        self.file,
                        expr.span.start as usize,
                        property.span.end as usize,
                    );
                    expr = Expr {
                        kind: ExprKind::Member {
                            object: Box::new(expr),
                            property: property.name,
                            optional: false,
                        },
                        span,
                    };
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.parse_expr(0)?;
                    let close = self.expect(TokenKind::RBracket, "expected ']'")?;
                    let span = Span::new(self.file, expr.span.start as usize, close.end as usize);
                    expr = Expr {
                        kind: ExprKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                            optional: false,
                        },
                        span,
                    };
                }
                TokenKind::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    while self.kind() != TokenKind::RParen && self.kind() != TokenKind::Eof {
                        args.push(self.parse_expr(0)?);
                        if !self.eat(TokenKind::Comma) {
                            break;
                        }
                    }
                    let close = self.expect(TokenKind::RParen, "expected ')' after arguments")?;
                    let span = Span::new(self.file, expr.span.start as usize, close.end as usize);
                    expr = Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                        span,
                    };
                }
                _ => break,
            }
        }
        Some(expr)
    }

    pub(super) fn parse_primary(&mut self) -> Option<Expr> {
        let token = self.current().clone();
        let (kind, end) = match self.kind() {
            TokenKind::Number => {
                self.bump();
                let value = decode_number(&token.lexeme);
                (ExprKind::Number(value), token.end)
            }
            TokenKind::String => {
                self.bump();
                (ExprKind::String(decode_string(&token.lexeme)), token.end)
            }
            TokenKind::True | TokenKind::False => {
                self.bump();
                (ExprKind::Bool(token.kind == TokenKind::True), token.end)
            }
            TokenKind::Null => {
                self.bump();
                (ExprKind::Null, token.end)
            }
            TokenKind::Ident => return self.parse_primary_ident(&token),
            TokenKind::LBracket => {
                self.bump();
                let mut items = Vec::new();
                while self.kind() != TokenKind::RBracket && self.kind() != TokenKind::Eof {
                    items.push(self.parse_expr(0)?);
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.expect(TokenKind::RBracket, "expected ']'")?;
                let span = Span::new(self.file, token.start as usize, close.end as usize);
                return Some(Expr {
                    kind: ExprKind::ArrayLit(items),
                    span,
                });
            }
            TokenKind::LBrace => {
                self.bump();
                let mut fields = Vec::new();
                while self.kind() != TokenKind::RBrace && self.kind() != TokenKind::Eof {
                    let field_start = self.current().start;
                    let name = self.parse_ident("expected a property name")?;
                    self.expect(TokenKind::Colon, "expected ':' after property name")?;
                    let value = self.parse_expr(0)?;
                    let field_end = self.previous_end(field_start as usize);
                    fields.push(ObjectField {
                        name: name.name,
                        value,
                        span: Span::new(self.file, field_start as usize, field_end),
                    });
                    if !self.eat(TokenKind::Comma)
                        && !self.eat(TokenKind::Semicolon)
                        && self.kind() != TokenKind::RBrace
                    {
                        self.error("expected ',' or ';' after property", self.current_span());
                    }
                }
                let close = self.expect(TokenKind::RBrace, "expected '}'")?;
                let span = Span::new(self.file, token.start as usize, close.end as usize);
                return Some(Expr {
                    kind: ExprKind::ObjectLit(fields),
                    span,
                });
            }
            TokenKind::LParen => {
                let start = token.start as usize;
                if self.arrow_ahead() {
                    return self.parse_paren_arrow(start);
                }
                self.bump();
                let inner = self.parse_expr(0)?;
                let close = self.expect(TokenKind::RParen, "expected ')'")?;
                let span = Span::new(self.file, token.start as usize, close.end as usize);
                return Some(Expr {
                    kind: ExprKind::Paren(Box::new(inner)),
                    span,
                });
            }
            _ => {
                self.error("expected an expression", self.current_span());
                return None;
            }
        };
        Some(Expr {
            kind,
            span: Span::new(self.file, token.start as usize, end as usize),
        })
    }
}
