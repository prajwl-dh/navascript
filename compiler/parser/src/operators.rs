//! Parsing helpers for compound assignment, updates and ternary expressions.

use navascript_ast::{AssignOp, Expr, ExprKind, Span, UpdateOp};
use navascript_lexer::TokenKind;

use super::parser::Parser;

impl Parser<'_> {
    pub(super) fn compound_assign_op(kind: TokenKind) -> Option<AssignOp> {
        match kind {
            TokenKind::PlusEq => Some(AssignOp::Add),
            TokenKind::MinusEq => Some(AssignOp::Sub),
            TokenKind::StarEq => Some(AssignOp::Mul),
            TokenKind::SlashEq => Some(AssignOp::Div),
            TokenKind::PercentEq => Some(AssignOp::Mod),
            _ => None,
        }
    }

    /// Parse the `? a : b` tail of a ternary expression.
    pub(super) fn parse_ternary_tail(&mut self, condition: Expr) -> Option<Expr> {
        self.bump(); // `?`
        let then_expr = self.parse_expr(0)?;
        self.expect(TokenKind::Colon, "expected ':' in ternary expression")?;
        let else_expr = self.parse_expr(0)?;
        let span = Span::new(
            self.file,
            condition.span.start as usize,
            else_expr.span.end as usize,
        );
        Some(Expr {
            kind: ExprKind::Ternary {
                condition: Box::new(condition),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            },
            span,
        })
    }

    /// Parse a prefix `++` / `--` when present.
    pub(super) fn parse_prefix_update(&mut self) -> Option<Option<Expr>> {
        let op = match self.kind() {
            TokenKind::PlusPlus => UpdateOp::Inc,
            TokenKind::MinusMinus => UpdateOp::Dec,
            _ => return Some(None),
        };
        let start = self.current().start;
        self.bump();
        let operand = self.parse_unary()?;
        let span = Span::new(self.file, start as usize, operand.span.end as usize);
        Some(Some(Expr {
            kind: ExprKind::Update {
                op,
                operand: Box::new(operand),
                prefix: true,
            },
            span,
        }))
    }

    /// Parse a postfix `++` / `--` when present.
    pub(super) fn parse_postfix_update(&mut self, expr: &Expr) -> Option<Option<Expr>> {
        let op = match self.kind() {
            TokenKind::PlusPlus => UpdateOp::Inc,
            TokenKind::MinusMinus => UpdateOp::Dec,
            _ => return Some(None),
        };
        self.bump();
        let span = Span::new(
            self.file,
            expr.span.start as usize,
            self.previous_end(expr.span.end as usize),
        );
        Some(Some(Expr {
            kind: ExprKind::Update {
                op,
                operand: Box::new(expr.clone()),
                prefix: false,
            },
            span,
        }))
    }
}
