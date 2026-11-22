//! Type-expression and type-declaration parsing.

use nava_ast::{InterfaceDecl, Span, TypeAliasDecl, TypeAnnotation, TypeExpr, TypeField};
use nava_lexer::TokenKind;

use super::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn parse_type_annotation(&mut self) -> Option<TypeAnnotation> {
        let start = self.current().start;
        let expr = self.parse_type_expr()?;
        let end = self.previous_end(start as usize);
        Some(TypeAnnotation {
            expr,
            span: Span::new(self.file, start as usize, end),
        })
    }

    /// Union types bind loosest; intersections bind tighter.
    pub(super) fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        let mut members = vec![self.parse_intersection_type()?];
        while self.eat(TokenKind::Pipe) {
            members.push(self.parse_intersection_type()?);
        }
        if members.len() == 1 {
            Some(members.pop().unwrap())
        } else {
            Some(TypeExpr::Union(members))
        }
    }

    fn parse_intersection_type(&mut self) -> Option<TypeExpr> {
        let mut members = vec![self.parse_postfix_type()?];
        while self.eat(TokenKind::Amp) {
            members.push(self.parse_postfix_type()?);
        }
        if members.len() == 1 {
            Some(members.pop().unwrap())
        } else {
            Some(TypeExpr::Intersection(members))
        }
    }

    fn parse_postfix_type(&mut self) -> Option<TypeExpr> {
        let mut base = match self.kind() {
            TokenKind::Ident | TokenKind::Void | TokenKind::Null => {
                let token = self.current().clone();
                self.bump();
                if self.kind() == TokenKind::Lt {
                    TypeExpr::Generic {
                        name: token.lexeme,
                        args: self.parse_generic_args()?,
                    }
                } else {
                    TypeExpr::Named(token.lexeme)
                }
            }
            TokenKind::LBrace => self.parse_object_type()?,
            TokenKind::LParen => self.parse_function_type()?,
            _ => {
                self.error("expected a type", self.current_span());
                return None;
            }
        };
        while self.eat(TokenKind::LBracket) {
            self.expect(TokenKind::RBracket, "expected ']' after '['")?;
            base = TypeExpr::Array(Box::new(base));
        }
        Some(base)
    }

    /// Parse a `<A, B>` list, with the current token on `<`. Shared by generic
    /// type references and explicit call type arguments. `>>` lexes as two
    /// `Gt` tokens, so nested generics need no special token.
    pub(super) fn parse_angle_args(&mut self) -> Option<Vec<TypeExpr>> {
        self.expect(TokenKind::Lt, "expected '<'")?;
        let mut args = Vec::new();
        while self.kind() != TokenKind::Gt && self.kind() != TokenKind::Eof {
            args.push(self.parse_type_expr()?);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Gt, "expected '>' after type arguments")?;
        Some(args)
    }

    pub(super) fn parse_generic_args(&mut self) -> Option<Vec<TypeExpr>> {
        self.parse_angle_args()
    }

    /// Parse a declaration's `<T, U>` parameter list, if present.
    pub(super) fn parse_decl_type_params(&mut self) -> Option<Vec<nava_ast::Ident>> {
        if self.kind() != TokenKind::Lt {
            return Some(Vec::new());
        }
        self.bump();
        let mut params = Vec::new();
        while self.kind() != TokenKind::Gt && self.kind() != TokenKind::Eof {
            params.push(self.parse_ident("expected a type parameter name")?);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Gt, "expected '>' after type parameters")?;
        Some(params)
    }

    pub(super) fn parse_object_type(&mut self) -> Option<TypeExpr> {
        self.expect(TokenKind::LBrace, "expected '{'")?;
        let fields = self.parse_type_fields();
        self.expect(TokenKind::RBrace, "expected '}'")?;
        Some(TypeExpr::Object(fields))
    }

    /// Function-type parameters may be named (`n: number`) or bare (`number`).
    pub(super) fn parse_type_param(&mut self) -> Option<TypeExpr> {
        if self.kind() == TokenKind::Ident
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.kind == TokenKind::Colon)
        {
            self.bump();
            self.bump();
        }
        self.parse_type_expr()
    }

    pub(super) fn parse_type_params(&mut self) -> Option<Vec<TypeExpr>> {
        self.expect(TokenKind::LParen, "expected '('")?;
        let mut params = Vec::new();
        while self.kind() != TokenKind::RParen && self.kind() != TokenKind::Eof {
            params.push(self.parse_type_param()?);
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected ')'")?;
        Some(params)
    }

    pub(super) fn parse_function_type(&mut self) -> Option<TypeExpr> {
        let params = self.parse_type_params()?;
        self.expect(TokenKind::Arrow, "expected '=>' in function type")?;
        let ret = self.parse_type_expr()?;
        Some(TypeExpr::Function {
            params,
            ret: Box::new(ret),
        })
    }

    pub(super) fn parse_type_fields(&mut self) -> Vec<TypeField> {
        let mut fields = Vec::new();
        while self.kind() != TokenKind::RBrace && self.kind() != TokenKind::Eof {
            let before = self.pos;
            let start = self.current().start;
            let Some(name) = self.parse_ident("expected a property name") else {
                self.recover();
                if self.pos == before {
                    self.bump();
                }
                continue;
            };
            let optional = self.eat(TokenKind::Question);
            let ty = if self.kind() == TokenKind::LParen {
                self.parse_method_type()
            } else {
                self.expect(TokenKind::Colon, "expected ':' after property name")
                    .and_then(|_| self.parse_type_expr())
                    .map(|ty| {
                        if optional {
                            TypeExpr::Union(vec![ty, TypeExpr::Named("null".to_owned())])
                        } else {
                            ty
                        }
                    })
            };
            if !self.eat(TokenKind::Semicolon)
                && !self.eat(TokenKind::Comma)
                && self.kind() != TokenKind::RBrace
            {
                self.error("expected ';' or ',' after property", self.current_span());
            }
            if let Some(ty) = ty {
                let end = self.previous_end(start as usize);
                fields.push(TypeField {
                    name: name.name,
                    ty,
                    optional,
                    span: Span::new(self.file, start as usize, end),
                });
            }
        }
        fields
    }

    pub(super) fn parse_method_type(&mut self) -> Option<TypeExpr> {
        let params = self.parse_type_params()?;
        let ret = if self.eat(TokenKind::Colon) {
            self.parse_type_expr()?
        } else {
            TypeExpr::Named("void".to_owned())
        };
        Some(TypeExpr::Function {
            params,
            ret: Box::new(ret),
        })
    }

    pub(super) fn parse_type_alias(&mut self) -> Option<TypeAliasDecl> {
        let start = self.current().start;
        self.bump(); // `type`
        let name = self.parse_ident("expected a type alias name")?;
        self.expect(TokenKind::Eq, "expected '=' after type alias name")?;
        let ty = self.parse_type_expr()?;
        self.expect_semicolon();
        let end = self.previous_end(start as usize);
        Some(TypeAliasDecl {
            name,
            ty,
            span: Span::new(self.file, start as usize, end),
        })
    }

    pub(super) fn parse_interface(&mut self) -> Option<InterfaceDecl> {
        let start = self.current().start;
        self.bump(); // `interface`
        let name = self.parse_ident("expected an interface name")?;
        let type_params = self.parse_decl_type_params()?;
        self.expect(TokenKind::LBrace, "expected '{' after interface name")?;
        let fields = self.parse_type_fields();
        self.expect(TokenKind::RBrace, "expected '}'")?;
        let end = self.previous_end(start as usize);
        Some(InterfaceDecl {
            name,
            type_params,
            fields,
            span: Span::new(self.file, start as usize, end),
        })
    }
}
