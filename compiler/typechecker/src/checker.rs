//! The type checker: one `check` seam hiding scopes, inference and checking.

use std::collections::{BTreeMap, HashMap};

use nava_ast::{
    BinaryOp, Block, Expr, ExprKind, FileId, FunctionDecl, Ident, IfStmt, Item, ItemKind, Program,
    Span, Stmt, StmtKind, TypeField, UnaryOp, VariableDecl,
};
use nava_diagnostics::{Diagnostic, Diagnostics, LineIndex};
use nava_symbol::{Symbol, SymbolKind, SymbolTable, Type};

use super::types::{AliasTable, is_assignable, resolve_named_type, resolve_type};

/// Result of checking: diagnostics plus the type information IR lowering needs.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckOutput {
    pub diagnostics: Diagnostics,
    pub typed: TypedProgram,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    /// Expression types keyed by the start byte of the expression's span.
    pub types: HashMap<u32, Type>,
    pub symbols: SymbolTable,
}

pub fn check(program: &Program, source: &str) -> CheckOutput {
    let mut checker = Checker::new(program, source);
    checker.seed_builtins();
    checker.collect_aliases();
    checker.check_alias_cycles();
    checker.collect_declarations();
    for item in &program.items {
        checker.check_item(item);
    }
    CheckOutput {
        diagnostics: Diagnostics::new(checker.diagnostics),
        typed: TypedProgram {
            types: checker.types,
            symbols: checker.symbols,
        },
    }
}

struct Checker<'a> {
    program: &'a Program,
    lines: LineIndex,
    symbols: SymbolTable,
    aliases: AliasTable,
    diagnostics: Vec<Diagnostic>,
    types: HashMap<u32, Type>,
}

impl<'a> Checker<'a> {
    fn new(program: &'a Program, source: &str) -> Self {
        Self {
            program,
            lines: LineIndex::new(source),
            symbols: SymbolTable::new(),
            aliases: AliasTable::new(),
            diagnostics: Vec::new(),
            types: HashMap::new(),
        }
    }

    /// Minimal built-in surface needed by the fibonacci acceptance program.
    /// The real stdlib arrives in Phase 5.
    fn seed_builtins(&mut self) {
        let mut console = BTreeMap::new();
        console.insert(
            "log".to_owned(),
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        );
        self.symbols.insert(Symbol {
            name: "console".to_owned(),
            kind: SymbolKind::Variable,
            ty: Type::Object(console),
            span: Span::new(FileId(0), 0, 0),
        });
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message, span, &self.lines));
    }

    fn collect_aliases(&mut self) {
        for item in &self.program.items {
            match &item.kind {
                ItemKind::TypeAlias(alias) => {
                    let ty = resolve_type(&alias.ty, &self.aliases);
                    self.aliases.insert(alias.name.name.clone(), ty);
                }
                ItemKind::Interface(interface) => {
                    let fields = self.resolve_fields(&interface.fields);
                    self.aliases
                        .insert(interface.name.name.clone(), Type::Object(fields));
                }
                _ => {}
            }
        }
    }

    fn check_alias_cycles(&mut self) {
        for item in &self.program.items {
            if let ItemKind::TypeAlias(alias) = &item.kind {
                let resolved =
                    resolve_named_type(&Type::Named(alias.name.name.clone()), &self.aliases);
                if resolved == Type::Error {
                    self.error(
                        format!("circular type alias '{}'", alias.name.name),
                        alias.span,
                    );
                }
            }
        }
    }

    fn resolve_fields(&self, fields: &[TypeField]) -> BTreeMap<String, Type> {
        fields
            .iter()
            .map(|field| (field.name.clone(), resolve_type(&field.ty, &self.aliases)))
            .collect()
    }

    fn collect_declarations(&mut self) {
        for item in &self.program.items {
            match &item.kind {
                ItemKind::Variable(decl) => {
                    let ty = decl
                        .ty
                        .as_ref()
                        .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
                        .unwrap_or(Type::Unknown);
                    self.declare(&decl.name, ty, SymbolKind::Variable);
                }
                ItemKind::Function(decl) => self.declare_function(decl),
                _ => {}
            }
        }
    }

    fn declare(&mut self, name: &Ident, ty: Type, kind: SymbolKind) {
        if self
            .symbols
            .insert(Symbol {
                name: name.name.clone(),
                kind,
                ty,
                span: name.span,
            })
            .is_some()
        {
            self.error(format!("duplicate declaration '{}'", name.name), name.span);
        }
    }

    fn declare_function(&mut self, decl: &FunctionDecl) {
        let params: Vec<Type> = decl
            .params
            .iter()
            .map(|param| {
                param
                    .ty
                    .as_ref()
                    .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
                    .unwrap_or(Type::Unknown)
            })
            .collect();
        let placeholder = Type::Function {
            params: params.clone(),
            ret: Box::new(Type::Unknown),
        };
        self.declare(&decl.name, placeholder, SymbolKind::Function);

        // Return types are inferred later, in `check_function`, when parameters
        // and every top-level function are in scope. Registering every
        // signature first also lets later declarations call earlier ones.
        let ret = decl
            .return_ty
            .as_ref()
            .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
            .unwrap_or(Type::Unknown);
        self.symbols.insert(Symbol {
            name: decl.name.name.clone(),
            kind: SymbolKind::Function,
            ty: Type::Function {
                params,
                ret: Box::new(ret),
            },
            span: decl.name.span,
        });
    }

    fn infer_return_type(&mut self, body: &Block) -> Type {
        let mut found: Option<Type> = None;
        self.collect_returns(&body.statements, &mut found);
        found.unwrap_or(Type::Void)
    }

    fn collect_returns(&mut self, statements: &[Stmt], found: &mut Option<Type>) {
        for statement in statements {
            match &statement.kind {
                StmtKind::Return(Some(expr)) => {
                    let ty = self.infer_expr(expr);
                    match found {
                        None => *found = Some(ty),
                        Some(existing) if *existing != ty => *found = Some(Type::Error),
                        _ => {}
                    }
                }
                StmtKind::If(if_stmt) => {
                    self.collect_returns(&if_stmt.then_branch.statements, found);
                    if let Some(nava_ast::ElseBranch::Block(block)) = &if_stmt.else_branch {
                        self.collect_returns(&block.statements, found);
                    }
                }
                StmtKind::Block(block) => self.collect_returns(&block.statements, found),
                _ => {}
            }
        }
    }

    fn check_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Variable(decl) => self.check_variable(decl, true),
            ItemKind::Function(decl) => self.check_function(decl),
            ItemKind::Expr(expr) => {
                self.infer_expr(expr);
            }
            ItemKind::TypeAlias(_) | ItemKind::Interface(_) => {}
        }
    }

    fn check_variable(&mut self, decl: &VariableDecl, top_level: bool) {
        let declared = decl
            .ty
            .as_ref()
            .map(|annotation| resolve_type(&annotation.expr, &self.aliases));
        let inferred = decl.init.as_ref().map(|expr| self.infer_expr(expr));

        if let (Some(expected), Some(actual)) = (&declared, &inferred) {
            if !is_assignable(expected, actual, &self.aliases) {
                self.error(
                    format!(
                        "cannot assign '{}' to '{}'",
                        actual.display(),
                        expected.display()
                    ),
                    decl.init.as_ref().expect("actual exists").span,
                );
            }
        }

        let final_ty = match (declared, inferred) {
            (Some(expected), _) => expected,
            (None, Some(actual)) => actual,
            (None, None) => Type::Unknown,
        };
        // Top-level declarations were inserted in the collection pass, where
        // duplicates were already reported. Locals are inserted here, so a
        // duplicate in the same scope is reported now.
        let previous = self.symbols.insert(Symbol {
            name: decl.name.name.clone(),
            kind: SymbolKind::Variable,
            ty: final_ty,
            span: decl.name.span,
        });
        if !top_level && previous.is_some() {
            self.error(
                format!("duplicate declaration '{}'", decl.name.name),
                decl.name.span,
            );
        }
    }

    fn check_function(&mut self, decl: &FunctionDecl) {
        self.symbols.enter_scope();
        for param in &decl.params {
            let ty = param
                .ty
                .as_ref()
                .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
                .unwrap_or(Type::Unknown);
            self.symbols.insert(Symbol {
                name: param.name.name.clone(),
                kind: SymbolKind::Parameter,
                ty,
                span: param.name.span,
            });
        }
        let expected_ret = decl
            .return_ty
            .as_ref()
            .map(|annotation| resolve_type(&annotation.expr, &self.aliases))
            .unwrap_or_else(|| self.infer_return_type(&decl.body));

        // Refresh the global signature so later functions see the inferred
        // return type rather than the placeholder registered in pass one.
        if let Some(Type::Function { params, .. }) = self
            .symbols
            .lookup(&decl.name.name)
            .map(|symbol| symbol.ty.clone())
        {
            self.symbols.insert(Symbol {
                name: decl.name.name.clone(),
                kind: SymbolKind::Function,
                ty: Type::Function {
                    params,
                    ret: Box::new(expected_ret.clone()),
                },
                span: decl.name.span,
            });
        }

        self.check_block(&decl.body, Some(&expected_ret));
        self.symbols.leave_scope();
    }

    fn check_block(&mut self, block: &Block, expected_ret: Option<&Type>) {
        self.symbols.enter_scope();
        for statement in &block.statements {
            self.check_stmt(statement, expected_ret);
        }
        self.symbols.leave_scope();
    }

    fn check_stmt(&mut self, statement: &Stmt, expected_ret: Option<&Type>) {
        match &statement.kind {
            StmtKind::Return(value) => {
                let actual = value
                    .as_ref()
                    .map(|expr| self.infer_expr(expr))
                    .unwrap_or(Type::Void);
                if let Some(expected) = expected_ret {
                    if !is_assignable(expected, &actual, &self.aliases) {
                        self.error(
                            format!(
                                "return type '{}' is not assignable to '{}'",
                                actual.display(),
                                expected.display()
                            ),
                            statement.span,
                        );
                    }
                }
            }
            StmtKind::Variable(decl) => self.check_variable(decl, false),
            StmtKind::If(if_stmt) => self.check_if(if_stmt, expected_ret),
            StmtKind::Block(block) => self.check_block(block, expected_ret),
            StmtKind::Expr(expr) => {
                self.infer_expr(expr);
            }
            StmtKind::Empty => {}
        }
    }

    fn check_if(&mut self, if_stmt: &IfStmt, expected_ret: Option<&Type>) {
        self.infer_expr(&if_stmt.condition);
        self.check_block(&if_stmt.then_branch, expected_ret);
        match &if_stmt.else_branch {
            Some(nava_ast::ElseBranch::If(nested)) => self.check_if(nested, expected_ret),
            Some(nava_ast::ElseBranch::Block(block)) => self.check_block(block, expected_ret),
            None => {}
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> Type {
        let ty = match &expr.kind {
            ExprKind::Number(_) => Type::Number,
            ExprKind::String(_) => Type::String,
            ExprKind::Bool(_) => Type::Boolean,
            ExprKind::Null => Type::Null,
            ExprKind::Ident(name) => match self.symbols.lookup(name) {
                Some(symbol) => symbol.ty.clone(),
                None => {
                    self.error(format!("unknown identifier '{name}'"), expr.span);
                    Type::Error
                }
            },
            ExprKind::Binary { op, left, right } => {
                let lhs = self.infer_expr(left);
                let rhs = self.infer_expr(right);
                self.infer_binary(*op, &lhs, &rhs, expr.span)
            }
            ExprKind::Unary { op, operand } => {
                let ty = self.infer_expr(operand);
                match op {
                    UnaryOp::Not => Type::Boolean,
                    UnaryOp::Neg | UnaryOp::Pos if matches!(ty, Type::Number | Type::Unknown) => {
                        Type::Number
                    }
                    UnaryOp::Neg | UnaryOp::Pos => {
                        self.error("unary operator expects a number", expr.span);
                        Type::Error
                    }
                }
            }
            ExprKind::Assign { target, value } => {
                let actual = self.infer_expr(value);
                self.check_assign_target(target, &actual);
                actual
            }
            ExprKind::Call { callee, args } => {
                let callee_ty = self.infer_expr(callee);
                self.infer_call(&callee_ty, args, expr.span)
            }
            ExprKind::Index { object, index } => {
                let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
                let index_ty = self.infer_expr(index);
                if !matches!(index_ty, Type::Number | Type::Unknown | Type::Error) {
                    self.error("array index must be a number", index.span);
                }
                match object_ty {
                    Type::Array(inner) => *inner,
                    Type::Error | Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            format!("cannot index a value of type '{}'", other.display()),
                            expr.span,
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::Member { object, property } => {
                let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
                match object_ty {
                    Type::Object(fields) => fields.get(property).cloned().unwrap_or_else(|| {
                        self.error(format!("property '{property}' does not exist"), expr.span);
                        Type::Error
                    }),
                    Type::Error | Type::Unknown => Type::Unknown,
                    other => {
                        self.error(
                            format!(
                                "property access requires an object, found '{}'",
                                other.display()
                            ),
                            expr.span,
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::ArrayLit(items) => {
                let element_types: Vec<Type> =
                    items.iter().map(|item| self.infer_expr(item)).collect();
                let mut unified = Type::Unknown;
                for (index, ty) in element_types.iter().enumerate() {
                    if index == 0 {
                        unified = ty.clone();
                    } else if unified != *ty {
                        self.error("array elements must have the same type", items[index].span);
                        unified = Type::Error;
                    }
                }
                Type::Array(Box::new(unified))
            }
            ExprKind::ObjectLit(fields) => {
                let mut object = BTreeMap::new();
                for field in fields {
                    object.insert(field.name.clone(), self.infer_expr(&field.value));
                }
                Type::Object(object)
            }
            ExprKind::Paren(inner) => self.infer_expr(inner),
        };
        self.types.insert(expr.span.start, ty.clone());
        ty
    }

    /// Check identifier, property and element assignment targets alike.
    fn check_assign_target(&mut self, target: &Expr, actual: &Type) {
        match &target.kind {
            ExprKind::Ident(name) => {
                match self.symbols.lookup(name).map(|symbol| symbol.ty.clone()) {
                    Some(expected) => self.check_assignment(&expected, actual, target.span),
                    None => self.error(format!("unknown identifier '{name}'"), target.span),
                }
            }
            ExprKind::Member { object, property } => {
                let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
                match object_ty {
                    Type::Object(fields) => match fields.get(property) {
                        Some(expected) => self.check_assignment(expected, actual, target.span),
                        None => {
                            self.error(format!("property '{property}' does not exist"), target.span)
                        }
                    },
                    Type::Error | Type::Unknown => {}
                    other => self.error(
                        format!("assignment requires an object, found '{}'", other.display()),
                        target.span,
                    ),
                }
            }
            ExprKind::Index { object, index } => {
                let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
                let index_ty = self.infer_expr(index);
                if !matches!(index_ty, Type::Number | Type::Unknown | Type::Error) {
                    self.error("array index must be a number", index.span);
                }
                match object_ty {
                    Type::Array(element) => self.check_assignment(&element, actual, target.span),
                    Type::Error | Type::Unknown => {}
                    other => self.error(
                        format!("cannot index a value of type '{}'", other.display()),
                        target.span,
                    ),
                }
            }
            _ => self.error("invalid assignment target", target.span),
        }
    }

    fn check_assignment(&mut self, expected: &Type, actual: &Type, span: Span) {
        if !is_assignable(expected, actual, &self.aliases) {
            let expected = resolve_named_type(expected, &self.aliases);
            self.error(
                format!(
                    "cannot assign '{}' to '{}'",
                    actual.display(),
                    expected.display()
                ),
                span,
            );
        }
    }

    fn infer_binary(&mut self, op: BinaryOp, lhs: &Type, rhs: &Type, span: Span) -> Type {
        match op {
            BinaryOp::Add if matches!(lhs, Type::String) || matches!(rhs, Type::String) => {
                Type::String
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                if !matches!(lhs, Type::Number | Type::Unknown | Type::Error)
                    || !matches!(rhs, Type::Number | Type::Unknown | Type::Error)
                {
                    self.error("arithmetic operator expects numbers", span);
                    Type::Error
                } else {
                    Type::Number
                }
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let numeric = matches!(lhs, Type::Number | Type::Unknown | Type::Error)
                    && matches!(rhs, Type::Number | Type::Unknown | Type::Error);
                let strings = matches!(lhs, Type::String) && matches!(rhs, Type::String);
                if numeric || strings {
                    Type::Boolean
                } else {
                    self.error("comparison requires two numbers or two strings", span);
                    Type::Error
                }
            }
            BinaryOp::Eq
            | BinaryOp::StrictEq
            | BinaryOp::NotEq
            | BinaryOp::StrictNotEq
            | BinaryOp::And
            | BinaryOp::Or => Type::Boolean,
        }
    }

    fn infer_call(&mut self, callee: &Type, args: &[Expr], span: Span) -> Type {
        let resolved = resolve_named_type(callee, &self.aliases);
        let Type::Function { params, ret } = resolved else {
            self.error(
                format!("cannot call a value of type '{}'", callee.display()),
                span,
            );
            return Type::Error;
        };
        if params.len() != args.len() {
            self.error(
                format!("expected {} arguments, found {}", params.len(), args.len()),
                span,
            );
        }
        for (expected, arg) in params.iter().zip(args) {
            let actual = self.infer_expr(arg);
            if !is_assignable(expected, &actual, &self.aliases) {
                self.error(
                    format!(
                        "argument has type '{}', expected '{}'",
                        actual.display(),
                        expected.display()
                    ),
                    arg.span,
                );
            }
        }
        *ret
    }
}
