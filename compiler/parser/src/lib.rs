//! Nava parser — the deep front-end module.
//!
//! Its external seam is [`parse`]: source text in, `ParseOutput` out. The
//! lexer is a private internal seam; tokenization, AST construction, span
//! mapping, error recovery and diagnostic ordering are all hidden here.

use std::fmt;

use nava_ast::{
    BinaryOp, Block, ElseBranch, Expr, ExprKind, FileId, FunctionDecl, Ident, IfStmt, Item,
    ItemKind, Param, Program, Span, Stmt, StmtKind, TypeAnnotation, UnaryOp, VariableDecl,
};
use nava_lexer::{Token, TokenKind, tokenize};

/// Severity of a front-end diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => f.write_str("error"),
            Severity::Warning => f.write_str("warning"),
        }
    }
}

/// A diagnostic with a source position derived from its span.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub line: u32,
    pub column: u32,
}

/// Deduplicated, source-ordered diagnostics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    fn new(mut items: Vec<Diagnostic>) -> Self {
        items.sort_by(|a, b| {
            a.span
                .start
                .cmp(&b.span.start)
                .then(severity_rank(a.severity).cmp(&severity_rank(b.severity)))
                .then(a.message.cmp(&b.message))
        });
        items.dedup_by(|a, b| {
            a.span == b.span && a.severity == b.severity && a.message == b.message
        });
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter()
    }

    /// Render diagnostics with a source snippet and caret for each one.
    pub fn render(&self, source: &str) -> String {
        let mut out = String::new();
        let line_starts = line_start_offsets(source);
        for diagnostic in &self.items {
            out.push_str(&format!(
                "<input>:{}:{}: {}: {}\n",
                diagnostic.line, diagnostic.column, diagnostic.severity, diagnostic.message
            ));
            let line_text = slice_line(source, &line_starts, diagnostic.line);
            out.push_str("  ");
            out.push_str(line_text);
            out.push('\n');
            out.push_str("  ");
            out.push_str(&" ".repeat(diagnostic.column.saturating_sub(1) as usize));
            let width = diagnostic.span.len().max(1) as usize;
            out.push_str(&"^".repeat(width));
            out.push('\n');
        }
        out
    }
}

fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 0,
        Severity::Warning => 1,
    }
}

/// Slice one line out of `source` using the shared line-start table.
fn slice_line<'a>(source: &'a str, line_starts: &[u32], line: u32) -> &'a str {
    let index = (line as usize).saturating_sub(1);
    let Some(&start) = line_starts.get(index) else {
        return "";
    };
    let start = start as usize;
    let end = source[start..]
        .find('\n')
        .map_or(source.len(), |offset| start + offset);
    &source[start..end]
}

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
    let line_starts = line_start_offsets(source);
    let mut parser = Parser {
        source,
        tokens: lexed.tokens,
        pos: 0,
        file,
        line_starts,
        diagnostics: Vec::new(),
        depth: 0,
    };

    for error in lexed.errors {
        let (line, column) = parser.position(error.start);
        parser.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: error.message,
            span: Span::new(file, error.start as usize, error.end as usize),
            line,
            column,
        });
    }

    let program = parser.parse_program();
    ParseOutput {
        program,
        diagnostics: Diagnostics::new(parser.diagnostics),
    }
}

fn line_start_offsets(source: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push((index + 1) as u32);
        }
    }
    starts
}

/// Keeps `parse` infallible: deeply nested input is diagnosed, not overflowed.
const MAX_DEPTH: u32 = 128;

struct Parser<'a> {
    source: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    file: FileId,
    line_starts: Vec<u32>,
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
        let token = match self.kind() {
            TokenKind::Ident | TokenKind::Void => self.current().clone(),
            _ => {
                self.error("expected a type name", self.current_span());
                return None;
            }
        };
        self.bump();
        let span = token_span(&token, self.file);
        Some(TypeAnnotation {
            name: token.lexeme,
            span,
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

    fn position(&self, offset: u32) -> (u32, u32) {
        let line = self.line_starts.partition_point(|start| *start <= offset) as u32;
        let column = offset - self.line_starts[(line - 1) as usize] + 1;
        (line, column)
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        let (line, column) = self.position(span.start);
        self.diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span,
            line,
            column,
        });
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(source: &str) -> ParseOutput {
        let output = parse(source);
        assert!(
            !output.diagnostics.has_errors(),
            "unexpected diagnostics: {}",
            output.diagnostics.render(source)
        );
        output
    }

    #[test]
    fn parses_variable_with_inference() {
        let output = ok("let answer = 42;");
        assert_eq!(output.program.items.len(), 1);
        let ItemKind::Variable(decl) = &output.program.items[0].kind else {
            panic!("expected a variable");
        };
        assert_eq!(decl.name.name, "answer");
        assert!(decl.ty.is_none());
        assert!(matches!(decl.init.as_ref().unwrap().kind, ExprKind::Number(n) if n == 42.0));
    }

    #[test]
    fn parses_typed_variable() {
        let output = ok("const name: string = \"nava\";");
        let ItemKind::Variable(decl) = &output.program.items[0].kind else {
            panic!("expected a variable");
        };
        assert_eq!(decl.ty.as_ref().unwrap().name, "string");
        assert!(matches!(&decl.init.as_ref().unwrap().kind, ExprKind::String(s) if s == "nava"));
    }

    #[test]
    fn parses_function_with_parameters_and_return_type() {
        let output = ok("function add(a: number, b: number): number { return a + b; }");
        let ItemKind::Function(decl) = &output.program.items[0].kind else {
            panic!("expected a function");
        };
        assert_eq!(decl.name.name, "add");
        assert_eq!(decl.params.len(), 2);
        assert_eq!(decl.return_ty.as_ref().unwrap().name, "number");
        assert_eq!(decl.body.statements.len(), 1);
        assert!(matches!(decl.body.statements[0].kind, StmtKind::Return(_)));
    }

    #[test]
    fn respects_operator_precedence() {
        let output = ok("let x = 1 + 2 * 3;");
        let ItemKind::Variable(decl) = &output.program.items[0].kind else {
            panic!("expected a variable");
        };
        let ExprKind::Binary { op, right, .. } = &decl.init.as_ref().unwrap().kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(*op, BinaryOp::Add);
        assert!(matches!(
            &right.kind,
            ExprKind::Binary {
                op: BinaryOp::Mul,
                ..
            }
        ));
    }

    #[test]
    fn parses_calls_and_member_access() {
        let output = ok("console.log(1);");
        let ItemKind::Expr(expr) = &output.program.items[0].kind else {
            panic!("expected an expression item");
        };
        let ExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected a call");
        };
        assert!(matches!(&callee.kind, ExprKind::Member { property, .. } if property == "log"));
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn parses_if_else_chain() {
        let output = ok(
            "function f() { if (a) { return 1; } else if (b) { return 2; } else { return 3; } }",
        );
        let ItemKind::Function(decl) = &output.program.items[0].kind else {
            panic!("expected a function");
        };
        let StmtKind::If(if_stmt) = &decl.body.statements[0].kind else {
            panic!("expected an if statement");
        };
        let Some(ElseBranch::If(second)) = &if_stmt.else_branch else {
            panic!("expected an else-if branch");
        };
        assert!(matches!(second.else_branch, Some(ElseBranch::Block(_))));
    }

    #[test]
    fn parses_the_fibonacci_acceptance_program() {
        let source = "function fibonacci(n: number): number { if (n <= 1) { return n; } return fibonacci(n - 1) + fibonacci(n - 2); } console.log(fibonacci(10));";
        let output = ok(source);
        assert_eq!(output.program.items.len(), 2);
        assert!(matches!(
            output.program.items[0].kind,
            ItemKind::Function(_)
        ));
        assert!(matches!(output.program.items[1].kind, ItemKind::Expr(_)));
    }

    #[test]
    fn reports_line_and_column_and_renders_caret() {
        let source = "let x = ;\nlet y = 1;";
        let output = parse(source);
        assert!(output.diagnostics.has_errors());
        let rendered = output.diagnostics.render(source);
        assert!(rendered.contains("<input>:1:9: error:"), "{rendered}");
        assert!(rendered.contains("^"), "{rendered}");
    }

    #[test]
    fn diagnostics_are_sorted_and_deduplicated() {
        let source = "let x = ;\nlet y = ;";
        let output = parse(source);
        let lines: Vec<u32> = output.diagnostics.iter().map(|d| d.line).collect();
        assert!(
            lines.windows(2).all(|pair| pair[0] <= pair[1]),
            "diagnostics out of order: {lines:?}"
        );
    }

    #[test]
    fn recovers_and_keeps_later_items() {
        let output = parse("let broken = ;\nlet good = 1;");
        assert!(output.diagnostics.has_errors());
        assert_eq!(output.program.items.len(), 1);
        let ItemKind::Variable(decl) = &output.program.items[0].kind else {
            panic!("expected a variable");
        };
        assert_eq!(decl.name.name, "good");
    }

    #[test]
    fn handles_empty_input() {
        let output = parse("");
        assert!(output.diagnostics.is_empty());
        assert!(output.program.items.is_empty());
    }

    #[test]
    fn parses_unbraced_if_and_else_bodies() {
        let output = ok("function f() { if (a) return 1; else return 2; }");
        let ItemKind::Function(decl) = &output.program.items[0].kind else {
            panic!("expected a function");
        };
        let StmtKind::If(if_stmt) = &decl.body.statements[0].kind else {
            panic!("expected an if statement");
        };
        assert_eq!(if_stmt.then_branch.statements.len(), 1);
        let Some(ElseBranch::Block(branch)) = &if_stmt.else_branch else {
            panic!("expected an else block");
        };
        assert_eq!(branch.statements.len(), 1);
    }

    #[test]
    fn deep_nesting_is_diagnosed_instead_of_overflowing() {
        let source = format!("let x = {}1{};", "(".repeat(300), ")".repeat(300));
        let output = parse(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|d| d.message.contains("nesting is too deep"))
        );
        let source = format!("let x = {}1;", "!".repeat(300));
        let output = parse(&source);
        assert!(output.diagnostics.has_errors());
    }

    #[test]
    fn diagnostics_deduplicate_and_break_ties_by_severity() {
        let span = Span::new(FileId(0), 1, 2);
        let error = Diagnostic {
            severity: Severity::Error,
            message: "same".to_owned(),
            span,
            line: 1,
            column: 2,
        };
        let warning = Diagnostic {
            severity: Severity::Warning,
            message: "same".to_owned(),
            span,
            line: 1,
            column: 2,
        };
        let diagnostics = Diagnostics::new(vec![warning.clone(), error.clone(), error.clone()]);
        let items: Vec<&Diagnostic> = diagnostics.iter().collect();
        assert_eq!(items.len(), 2, "duplicate was not removed");
        assert_eq!(
            items[0].severity,
            Severity::Error,
            "tie-break is not stable"
        );
        assert_eq!(items[1].severity, Severity::Warning);
    }

    #[test]
    fn program_has_a_debug_form() {
        let output = ok("let x = 1;");
        let debug = format!("{:?}", output.program);
        assert!(debug.contains("Variable"), "{debug}");
        assert!(debug.contains("Number"), "{debug}");
    }
}
