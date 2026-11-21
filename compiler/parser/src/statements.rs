//! Statement and control-flow parsing.

use nava_ast::{Block, ElseBranch, IfStmt, Span, Stmt, StmtKind};
use nava_lexer::TokenKind;

use super::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn parse_block(&mut self) -> Option<Block> {
        let open = self.expect(TokenKind::LBrace, "expected '{'")?;
        let statements = self.parse_until(
            |kind| matches!(kind, TokenKind::RBrace | TokenKind::Eof),
            Self::parse_stmt,
        );
        let close = self.expect(TokenKind::RBrace, "expected '}'")?;
        Some(Block {
            statements,
            span: Span::new(self.file, open.start as usize, close.end as usize),
        })
    }

    /// An `if`/`else` branch is either a block or one statement (TypeScript
    /// allows `if (x) return;`). A lone statement is wrapped in a synthetic
    /// block so the AST shape stays uniform.
    pub(super) fn parse_branch(&mut self) -> Option<Block> {
        if self.kind() == TokenKind::LBrace {
            return self.parse_block();
        }
        let statement = self.parse_stmt()?;
        let span = statement.span;
        Some(Block {
            statements: vec![statement],
            span,
        })
    }

    pub(super) fn parse_stmt(&mut self) -> Option<Stmt> {
        if !self.enter() {
            return None;
        }
        let result = self.parse_stmt_inner();
        self.leave();
        result
    }

    pub(super) fn parse_stmt_inner(&mut self) -> Option<Stmt> {
        let start = self.current().start;
        let kind = match self.kind() {
            TokenKind::Return => {
                self.bump();
                let value = if self.kind() == TokenKind::Semicolon || self.kind() == TokenKind::Eof
                {
                    None
                } else {
                    Some(self.parse_expr(0)?)
                };
                self.expect_semicolon();
                StmtKind::Return(value)
            }
            TokenKind::If => StmtKind::If(self.parse_if()?),
            TokenKind::LBrace => StmtKind::Block(self.parse_block()?),
            TokenKind::Let | TokenKind::Const | TokenKind::Var => {
                StmtKind::Variable(self.parse_variable_decl()?)
            }
            TokenKind::Semicolon => {
                self.bump();
                StmtKind::Empty
            }
            _ => {
                let expr = self.parse_expr(0)?;
                self.expect_semicolon();
                StmtKind::Expr(expr)
            }
        };
        let end = self.previous_end(start as usize);
        Some(Stmt {
            kind,
            span: Span::new(self.file, start as usize, end),
        })
    }

    pub(super) fn parse_if(&mut self) -> Option<IfStmt> {
        self.bump(); // `if`
        self.expect(TokenKind::LParen, "expected '(' after 'if'")?;
        let condition = self.parse_expr(0)?;
        self.expect(TokenKind::RParen, "expected ')' after condition")?;
        let then_branch = self.parse_branch()?;
        let else_branch = if self.eat(TokenKind::Else) {
            if self.kind() == TokenKind::If {
                Some(ElseBranch::If(Box::new(self.parse_if()?)))
            } else {
                Some(ElseBranch::Block(self.parse_branch()?))
            }
        } else {
            None
        };
        Some(IfStmt {
            condition,
            then_branch,
            else_branch,
        })
    }
}
