//! Nava parser — the deep front-end module.
//!
//! Its external seam is [`parse`]: source text in, `ParseOutput` out. The
//! lexer is a private internal seam; tokenization, AST construction, span
//! mapping, error recovery and diagnostic ordering are all hidden here.

use nava_ast::{
    BinaryOp, Block, ElseBranch, Expr, ExprKind, FileId, FunctionDecl, Ident, IfStmt,
    InterfaceDecl, Item, ItemKind, ObjectField, Param, Program, Span, Stmt, StmtKind,
    TypeAliasDecl, TypeAnnotation, TypeExpr, TypeField, UnaryOp, VariableDecl,
};
use nava_diagnostics::LineIndex;
pub use nava_diagnostics::{Diagnostic, Diagnostics, Severity};
use nava_lexer::{Token, TokenKind, tokenize};

/// The result of parsing: an owned program plus all diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutput {
    /// Best-effort program. It is partial when diagnostics contain errors.
    pub program: Program,
    pub diagnostics: Diagnostics,
}

/// Parse source text into a program and diagnostics.
///
/// Infallible and I/O-free: syntax errors become diagnostics, never `Err` or a
/// panic. The returned data is owned, so it outlives `source` and ports to Nava.
pub fn parse(source: &str) -> ParseOutput {
    let lexed = tokenize(source);
    let file = FileId(0);
    let lines = LineIndex::new(source);
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        file,
        lines,
        diagnostics: Vec::new(),
        depth: 0,
    };

    for error in lexed.errors {
        let span = Span::new(file, error.start as usize, error.end as usize);
        parser
            .diagnostics
            .push(Diagnostic::error(error.message, span, &parser.lines));
    }

    let program = parser.parse_program();
    ParseOutput {
        program,
        diagnostics: Diagnostics::new(parser.diagnostics),
    }
}

/// Keeps `parse` infallible: deeply nested input is diagnosed, not overflowed.
const MAX_DEPTH: u32 = 128;

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    file: FileId,
    lines: LineIndex,
    diagnostics: Vec<Diagnostic>,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn parse_program(&mut self) -> Program {
        let items = self.parse_until(|kind| kind == TokenKind::Eof, Self::parse_item);
        Program {
            items,
            span: Span::new(self.file, 0, self.source.len()),
        }
    }

    /// Shared list loop: parse one node, recover on failure, always progress.
    fn parse_until<T>(
        &mut self,
        done: impl Fn(TokenKind) -> bool,
        mut parse_one: impl FnMut(&mut Self) -> Option<T>,
    ) -> Vec<T> {
        let mut nodes = Vec::new();
        while !done(self.kind()) {
            let before = self.pos;
            if let Some(node) = parse_one(self) {
                nodes.push(node);
            } else {
                self.recover();
            }
            if self.pos == before {
                self.bump();
            }
        }
        nodes
    }

    fn enter(&mut self) -> bool {
        if self.depth >= MAX_DEPTH {
            self.error("expression nesting is too deep", self.current_span());
            return false;
        }
        self.depth += 1;
        true
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn parse_item(&mut self) -> Option<Item> {
        let start = self.current().start;
        let kind = match self.kind() {
            TokenKind::Let | TokenKind::Const | TokenKind::Var => {
                ItemKind::Variable(self.parse_variable_decl()?)
            }
            TokenKind::Function => ItemKind::Function(self.parse_function()?),
            TokenKind::Type => ItemKind::TypeAlias(self.parse_type_alias()?),
            TokenKind::Interface => ItemKind::Interface(self.parse_interface()?),
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

    fn parse_variable_decl(&mut self) -> Option<VariableDecl> {
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

    fn parse_function(&mut self) -> Option<FunctionDecl> {
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

    fn parse_type_annotation(&mut self) -> Option<TypeAnnotation> {
        let start = self.current().start;
        let expr = self.parse_type_expr()?;
        let end = self.previous_end(start as usize);
        Some(TypeAnnotation {
            expr,
            span: Span::new(self.file, start as usize, end),
        })
    }

    fn parse_type_expr(&mut self) -> Option<TypeExpr> {
        let mut base = match self.kind() {
            TokenKind::Ident | TokenKind::Void => {
                let token = self.current().clone();
                self.bump();
                TypeExpr::Named(token.lexeme)
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

    fn parse_object_type(&mut self) -> Option<TypeExpr> {
        self.expect(TokenKind::LBrace, "expected '{'")?;
        let fields = self.parse_type_fields();
        self.expect(TokenKind::RBrace, "expected '}'")?;
        Some(TypeExpr::Object(fields))
    }

    /// Function-type parameters may be named (`n: number`) or bare (`number`).
    fn parse_type_param(&mut self) -> Option<TypeExpr> {
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

    fn parse_type_params(&mut self) -> Option<Vec<TypeExpr>> {
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

    fn parse_function_type(&mut self) -> Option<TypeExpr> {
        let params = self.parse_type_params()?;
        self.expect(TokenKind::Arrow, "expected '=>' in function type")?;
        let ret = self.parse_type_expr()?;
        Some(TypeExpr::Function {
            params,
            ret: Box::new(ret),
        })
    }

    fn parse_type_fields(&mut self) -> Vec<TypeField> {
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
            let ty = if self.kind() == TokenKind::LParen {
                self.parse_method_type()
            } else {
                self.expect(TokenKind::Colon, "expected ':' after property name")
                    .and_then(|_| self.parse_type_expr())
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
                    span: Span::new(self.file, start as usize, end),
                });
            }
        }
        fields
    }

    fn parse_method_type(&mut self) -> Option<TypeExpr> {
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

    fn parse_type_alias(&mut self) -> Option<TypeAliasDecl> {
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

    fn parse_interface(&mut self) -> Option<InterfaceDecl> {
        let start = self.current().start;
        self.bump(); // `interface`
        let name = self.parse_ident("expected an interface name")?;
        self.expect(TokenKind::LBrace, "expected '{' after interface name")?;
        let fields = self.parse_type_fields();
        self.expect(TokenKind::RBrace, "expected '}'")?;
        let end = self.previous_end(start as usize);
        Some(InterfaceDecl {
            name,
            fields,
            span: Span::new(self.file, start as usize, end),
        })
    }

    fn parse_ident(&mut self, message: &str) -> Option<Ident> {
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

    fn parse_block(&mut self) -> Option<Block> {
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
    fn parse_branch(&mut self) -> Option<Block> {
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

    fn parse_stmt(&mut self) -> Option<Stmt> {
        if !self.enter() {
            return None;
        }
        let result = self.parse_stmt_inner();
        self.leave();
        result
    }

    fn parse_stmt_inner(&mut self) -> Option<Stmt> {
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

    fn parse_if(&mut self) -> Option<IfStmt> {
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

    fn parse_expr(&mut self, min_prec: u8) -> Option<Expr> {
        if !self.enter() {
            return None;
        }
        let result = self.parse_expr_inner(min_prec);
        self.leave();
        result
    }

    fn parse_expr_inner(&mut self, min_prec: u8) -> Option<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            if self.kind() == TokenKind::Eq && min_prec == 0 {
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
            let Some((op, prec)) = self.peek_binary_op() else {
                break;
            };
            if prec < min_prec {
                break;
            }
            self.bump();
            let right = self.parse_expr(prec + 1)?;
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
        Some(left)
    }

    fn peek_binary_op(&self) -> Option<(BinaryOp, u8)> {
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
            _ => return None,
        };
        let precedence = match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Eq | BinaryOp::StrictEq | BinaryOp::NotEq | BinaryOp::StrictNotEq => 3,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 4,
            BinaryOp::Add | BinaryOp::Sub => 5,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 6,
        };
        Some((op, precedence))
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        if !self.enter() {
            return None;
        }
        let result = self.parse_unary_inner();
        self.leave();
        result
    }

    fn parse_unary_inner(&mut self) -> Option<Expr> {
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

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
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

    fn parse_primary(&mut self) -> Option<Expr> {
        let token = self.current().clone();
        let (kind, end) = match self.kind() {
            TokenKind::Number => {
                self.bump();
                let value = token.lexeme.parse::<f64>().unwrap_or(f64::NAN);
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
            TokenKind::Ident => {
                self.bump();
                (ExprKind::Ident(token.lexeme.clone()), token.end)
            }
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

    fn expect(&mut self, kind: TokenKind, message: &str) -> Option<Token> {
        if self.kind() == kind {
            let token = self.current().clone();
            self.bump();
            Some(token)
        } else {
            self.error(message, self.current_span());
            None
        }
    }

    fn expect_semicolon(&mut self) -> u32 {
        if self.kind() == TokenKind::Semicolon {
            let end = self.current().end;
            self.bump();
            end
        } else {
            self.error("expected ';'", self.current_span());
            self.current().start
        }
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.kind() == kind {
            self.bump();
            true
        } else {
            false
        }
    }

    fn recover(&mut self) {
        while !matches!(
            self.kind(),
            TokenKind::Semicolon
                | TokenKind::RBrace
                | TokenKind::Eof
                | TokenKind::Let
                | TokenKind::Const
                | TokenKind::Var
                | TokenKind::Function
                | TokenKind::Return
                | TokenKind::If
                | TokenKind::Else
        ) {
            self.bump();
        }
        if self.kind() == TokenKind::Semicolon {
            self.bump();
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn kind(&self) -> TokenKind {
        self.current().kind
    }

    fn current_span(&self) -> Span {
        token_span(self.current(), self.file)
    }

    fn bump(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn previous_end(&self, fallback: usize) -> usize {
        if self.pos > 0 {
            self.tokens[self.pos - 1].end as usize
        } else {
            fallback
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        let diagnostic = Diagnostic::error(message, span, &self.lines);
        self.diagnostics.push(diagnostic);
    }
}

fn token_span(token: &Token, file: FileId) -> Span {
    Span::new(file, token.start as usize, token.end as usize)
}

fn decode_string(lexeme: &str) -> String {
    if lexeme.len() < 2 {
        return String::new();
    }
    let inner = &lexeme[1..lexeme.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}
