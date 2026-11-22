//! Declarations, blocks and identifier parsing.

use nava_ast::{
    ClassDecl, ClassMember, FunctionDecl, Ident, Item, ItemKind, Param, Span, VariableDecl,
};
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
            TokenKind::Class => ItemKind::Class(self.parse_class()?),
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
        let type_params = self.parse_decl_type_params()?;
        let params = self.parse_params()?;
        let return_ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Some(FunctionDecl {
            name,
            type_params,
            params,
            return_ty,
            body,
        })
    }

    /// Parse `(name: Type, ...)` shared by functions, methods and constructors.
    pub(super) fn parse_params(&mut self) -> Option<Vec<Param>> {
        self.expect(TokenKind::LParen, "expected '(' after a name")?;
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
        Some(params)
    }

    pub(super) fn parse_class(&mut self) -> Option<ClassDecl> {
        let start = self.current().start;
        self.bump(); // `class`
        let name = self.parse_ident("expected a class name")?;
        let base = if self.eat(TokenKind::Extends) {
            Some(self.parse_ident("expected a base class name")?)
        } else {
            None
        };
        self.expect(TokenKind::LBrace, "expected '{' after class header")?;
        let mut members = Vec::new();
        while self.kind() != TokenKind::RBrace && self.kind() != TokenKind::Eof {
            if self.eat(TokenKind::Semicolon) {
                continue;
            }
            let before = self.pos;
            let member = if self.kind() == TokenKind::Ident {
                let member_name = self.parse_ident("expected a member name")?;
                if member_name.name == "constructor" && self.kind() == TokenKind::LParen {
                    ClassMember::Constructor {
                        params: self.parse_params()?,
                        body: self.parse_block()?,
                    }
                } else if self.kind() == TokenKind::LParen {
                    let params = self.parse_params()?;
                    let return_ty = if self.eat(TokenKind::Colon) {
                        Some(self.parse_type_annotation()?)
                    } else {
                        None
                    };
                    ClassMember::Method {
                        name: member_name,
                        params,
                        return_ty,
                        body: self.parse_block()?,
                    }
                } else {
                    let ty = if self.eat(TokenKind::Colon) {
                        Some(self.parse_type_annotation()?)
                    } else {
                        None
                    };
                    self.expect_semicolon();
                    ClassMember::Field {
                        name: member_name,
                        ty,
                    }
                }
            } else {
                self.error("expected a class member", self.current_span());
                self.recover();
                continue;
            };
            members.push(member);
            if self.pos == before {
                self.bump();
            }
        }
        self.expect(TokenKind::RBrace, "expected '}' after class body")?;
        let end = self.previous_end(start as usize);
        Some(ClassDecl {
            name,
            base,
            members,
            span: Span::new(self.file, start as usize, end),
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
