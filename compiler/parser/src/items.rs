//! Declarations, blocks and identifier parsing.

use nava_ast::{FunctionDecl, Ident, Item, ItemKind, Param, Span, VariableDecl};
use nava_lexer::TokenKind;

use super::parser::{Parser, token_span};

impl<'a> Parser<'a> {
    pub(super) fn parse_item(&mut self) -> Option<Item> {
        let start = self.current().start;
        let kind = match self.kind() {
            TokenKind::Let | TokenKind::Const | TokenKind::Var => {
                ItemKind::Variable(self.parse_variable_decl()?)
            }
            TokenKind::Function => ItemKind::Function(self.parse_function()?),
            TokenKind::Type => ItemKind::TypeAlias(self.parse_type_alias()?),
            TokenKind::Interface => ItemKind::Interface(self.parse_interface()?),
            TokenKind::While
            | TokenKind::Do
            | TokenKind::For
            | TokenKind::Break
            | TokenKind::Continue => ItemKind::Stmt(self.parse_stmt()?),
            _ => {
                let expr = self.parse_expr(0)?;
                self.expect_semicolon();
                ItemKind::Expr(expr)
            }
        };
        let end = self.previous_end(start as usize);
        Some(Item {
            kind,
            span: Span::new(self.file, start as usize, end),
        })
    }

    pub(super) fn parse_variable_decl(&mut self) -> Option<VariableDecl> {
        self.bump();
        let name = self.parse_ident("expected a variable name")?;
        let ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let init = if self.eat(TokenKind::Eq) {
            Some(self.parse_expr(0)?)
        } else {
            None
        };
        self.expect_semicolon();
        Some(VariableDecl { name, ty, init })
    }

    pub(super) fn parse_function(&mut self) -> Option<FunctionDecl> {
        self.bump(); // `function`
        let name = self.parse_ident("expected a function name")?;
        self.expect(TokenKind::LParen, "expected '(' after function name")?;
        let mut params = Vec::new();
        while self.kind() != TokenKind::RParen && self.kind() != TokenKind::Eof {
            let start = self.current().start;
            let param_name = self.parse_ident("expected a parameter name")?;
            let ty = if self.eat(TokenKind::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let end = self.current().end;
            params.push(Param {
                name: param_name,
                ty,
                span: Span::new(self.file, start as usize, end as usize),
            });
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected ')' after parameters")?;
        let return_ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Some(FunctionDecl {
            name,
            params,
            return_ty,
            body,
        })
    }

    pub(super) fn parse_ident(&mut self, message: &str) -> Option<Ident> {
        let token = match self.kind() {
            TokenKind::Ident => self.current().clone(),
            _ => {
                self.error(message, self.current_span());
                return None;
            }
        };
        self.bump();
        let span = token_span(&token, self.file);
        Some(Ident {
            name: token.lexeme,
            span,
        })
    }
}
