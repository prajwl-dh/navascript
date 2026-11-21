//! Statement and control-flow parsing.

use nava_ast::{
    Block, ElseBranch, ExprKind, ForOfStmt, ForStmt, Ident, IfStmt, Span, Stmt, StmtKind,
    VariableDecl, WhileStmt,
};
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
            TokenKind::While => StmtKind::While(self.parse_while()?),
            TokenKind::Do => StmtKind::DoWhile(self.parse_do_while()?),
            TokenKind::For => self.parse_for()?,
            TokenKind::Break => {
                self.bump();
                self.expect_semicolon();
                StmtKind::Break
            }
            TokenKind::Continue => {
                self.bump();
                self.expect_semicolon();
                StmtKind::Continue
            }
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

    pub(super) fn parse_while(&mut self) -> Option<WhileStmt> {
        self.bump(); // `while`
        self.expect(TokenKind::LParen, "expected '(' after 'while'")?;
        let condition = self.parse_expr(0)?;
        self.expect(TokenKind::RParen, "expected ')' after condition")?;
        let body = self.parse_branch()?;
        Some(WhileStmt { condition, body })
    }

    pub(super) fn parse_do_while(&mut self) -> Option<WhileStmt> {
        self.bump(); // `do`
        let body = self.parse_branch()?;
        self.expect(TokenKind::While, "expected 'while' after 'do' body")?;
        self.expect(TokenKind::LParen, "expected '(' after 'while'")?;
        let condition = self.parse_expr(0)?;
        self.expect(TokenKind::RParen, "expected ')' after condition")?;
        self.expect_semicolon();
        Some(WhileStmt { condition, body })
    }

    /// `for (init; condition; update)` or `for (let x of iterable)`. `of` is
    /// recognised contextually so it remains usable as an ordinary identifier.
    pub(super) fn parse_for(&mut self) -> Option<StmtKind> {
        let start = self.current().start as usize;
        self.bump(); // `for`
        self.expect(TokenKind::LParen, "expected '(' after 'for'")?;
        let init = if self.eat(TokenKind::Semicolon) {
            None
        } else if matches!(
            self.kind(),
            TokenKind::Let | TokenKind::Const | TokenKind::Var
        ) {
            self.bump(); // declaration keyword
            let name = self.parse_ident("expected a variable name")?;
            if self.kind() == TokenKind::Ident && self.current().lexeme == "of" {
                self.bump(); // contextual `of`
                let iterable = self.parse_expr(0)?;
                self.expect(TokenKind::RParen, "expected ')' after for-of header")?;
                let body = self.parse_branch()?;
                return Some(StmtKind::ForOf(ForOfStmt {
                    binding: name,
                    iterable,
                    body,
                    decl: true,
                }));
            }
            let ty = if self.eat(TokenKind::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let value = if self.eat(TokenKind::Eq) {
                Some(self.parse_expr(0)?)
            } else {
                None
            };
            self.expect_semicolon();
            let end = self.previous_end(start);
            Some(Box::new(Stmt {
                kind: StmtKind::Variable(VariableDecl {
                    name,
                    ty,
                    init: value,
                }),
                span: Span::new(self.file, start, end),
            }))
        } else {
            let expr = self.parse_expr(0)?;
            if self.kind() == TokenKind::Ident && self.current().lexeme == "of" {
                let binding = match &expr.kind {
                    ExprKind::Ident(name) => Ident {
                        name: name.clone(),
                        span: expr.span,
                    },
                    _ => {
                        self.error("for-of binding must be an identifier", expr.span);
                        Ident {
                            name: String::new(),
                            span: expr.span,
                        }
                    }
                };
                self.bump(); // contextual `of`
                let iterable = self.parse_expr(0)?;
                self.expect(TokenKind::RParen, "expected ')' after for-of header")?;
                let body = self.parse_branch()?;
                return Some(StmtKind::ForOf(ForOfStmt {
                    binding,
                    iterable,
                    body,
                    decl: false,
                }));
            }
            self.expect_semicolon();
            let span = expr.span;
            Some(Box::new(Stmt {
                kind: StmtKind::Expr(expr),
                span,
            }))
        };

        let condition = if self.kind() == TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expr(0)?)
        };
        self.expect_semicolon();
        let update = if self.kind() == TokenKind::RParen {
            None
        } else {
            Some(self.parse_expr(0)?)
        };
        self.expect(TokenKind::RParen, "expected ')' after for header")?;
        let body = self.parse_branch()?;
        Some(StmtKind::For(ForStmt {
            init,
            condition,
            update,
            body,
        }))
    }
}
