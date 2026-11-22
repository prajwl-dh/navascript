//! Alias collection, cycle detection and function signature registration.

use std::collections::BTreeMap;

use nava_ast::{Block, FunctionDecl, Ident, ItemKind, Stmt, StmtKind, TypeField};
use nava_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::{Subst, resolve_named_type};

impl<'a> Checker<'a> {
    pub(super) fn collect_aliases(&mut self) {
        for item in &self.program.items {
            match &item.kind {
                ItemKind::TypeAlias(alias) => {
                    let ty = self.resolve(&alias.ty, &Subst::new(), alias.span);
                    self.aliases.insert(alias.name.name.clone(), ty);
                }
                ItemKind::Interface(interface) => {
                    let subst = self.type_param_subst(&interface.type_params);
                    self.generic_params.insert(
                        interface.name.name.clone(),
                        interface
                            .type_params
                            .iter()
                            .map(|param| param.name.clone())
                            .collect(),
                    );
                    let fields = self.resolve_fields(&interface.fields, &subst);
                    self.aliases
                        .insert(interface.name.name.clone(), Type::Object(fields));
                }
                _ => {}
            }
        }
    }

    pub(super) fn check_alias_cycles(&mut self) {
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

    pub(super) fn resolve_fields(
        &mut self,
        fields: &[TypeField],
        subst: &Subst,
    ) -> BTreeMap<String, Type> {
        let mut resolved = BTreeMap::new();
        for field in fields {
            resolved.insert(
                field.name.clone(),
                self.resolve(&field.ty, subst, field.span),
            );
        }
        resolved
    }

    pub(super) fn collect_declarations(&mut self) {
        for item in &self.program.items {
            match &item.kind {
                ItemKind::Variable(decl) => {
                    let ty = decl
                        .ty
                        .as_ref()
                        .map(|annotation| {
                            self.resolve(&annotation.expr, &Subst::new(), decl.name.span)
                        })
                        .unwrap_or(Type::Unknown);
                    self.declare(&decl.name, ty, SymbolKind::Variable);
                }
                ItemKind::Function(decl) => self.declare_function(decl),
                _ => {}
            }
        }
    }

    pub(super) fn declare(&mut self, name: &Ident, ty: Type, kind: SymbolKind) {
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

    pub(super) fn declare_function(&mut self, decl: &FunctionDecl) {
        let subst = self.type_param_subst(&decl.type_params);
        self.generic_params.insert(
            decl.name.name.clone(),
            decl.type_params
                .iter()
                .map(|param| param.name.clone())
                .collect(),
        );
        let params: Vec<Type> = decl
            .params
            .iter()
            .map(|param| {
                param
                    .ty
                    .as_ref()
                    .map(|annotation| self.resolve(&annotation.expr, &subst, param.span))
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
            .map(|annotation| self.resolve(&annotation.expr, &subst, decl.name.span))
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

    pub(super) fn infer_return_type(&mut self, body: &Block) -> Type {
        let mut found: Option<Type> = None;
        self.collect_returns(&body.statements, &mut found);
        found.unwrap_or(Type::Void)
    }

    pub(super) fn collect_returns(&mut self, statements: &[Stmt], found: &mut Option<Type>) {
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
                StmtKind::While(stmt) | StmtKind::DoWhile(stmt) => {
                    self.collect_returns(&stmt.body.statements, found)
                }
                StmtKind::For(stmt) => self.collect_returns(&stmt.body.statements, found),
                StmtKind::ForOf(stmt) => self.collect_returns(&stmt.body.statements, found),
                StmtKind::Block(block) => self.collect_returns(&block.statements, found),
                _ => {}
            }
        }
    }
}
