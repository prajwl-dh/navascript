//! Control-flow narrowing of nullable union bindings.

use nava_ast::{BinaryOp, Expr, ExprKind};
use nava_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::resolve_named_type;

/// The refined types of a binding in the two branches of a condition.
pub(super) struct Narrowing {
    pub name: String,
    pub original: Type,
    pub then_ty: Type,
    pub else_ty: Type,
}

/// Drop `null` from a union, leaving the type of a definitely-non-null value.
fn non_null(ty: &Type) -> Type {
    match ty {
        Type::Union(members) => {
            let rest: Vec<Type> = members
                .iter()
                .filter(|member| !matches!(member, Type::Null))
                .cloned()
                .collect();
            match rest.len() {
                0 => Type::Null,
                1 => rest.into_iter().next().unwrap(),
                _ => Type::Union(rest),
            }
        }
        _ => ty.clone(),
    }
}

impl Checker<'_> {
    /// If `condition` tests a nullable binding, return its branch types.
    pub(super) fn narrow_binding(&self, condition: &Expr) -> Option<Narrowing> {
        match &condition.kind {
            // A conjunct narrows the then branch; a disjunct narrows the else
            // branch. The other branch keeps the binding's original type.
            ExprKind::Binary {
                op: BinaryOp::And,
                left,
                ..
            } => {
                let inner = self.narrow_binding(left)?;
                Some(Narrowing {
                    name: inner.name,
                    original: inner.original.clone(),
                    then_ty: inner.then_ty,
                    else_ty: inner.original,
                })
            }
            ExprKind::Binary {
                op: BinaryOp::Or,
                left,
                ..
            } => {
                let inner = self.narrow_binding(left)?;
                Some(Narrowing {
                    name: inner.name,
                    original: inner.original.clone(),
                    then_ty: inner.original,
                    else_ty: inner.else_ty,
                })
            }
            ExprKind::Binary { op, left, right }
                if matches!(
                    op,
                    BinaryOp::Eq | BinaryOp::StrictEq | BinaryOp::NotEq | BinaryOp::StrictNotEq
                ) =>
            {
                let negated = matches!(op, BinaryOp::NotEq | BinaryOp::StrictNotEq);
                let name = match (&left.kind, &right.kind) {
                    (ExprKind::Ident(name), ExprKind::Null)
                    | (ExprKind::Null, ExprKind::Ident(name)) => name.clone(),
                    _ => return None,
                };
                self.null_test(&name, negated)
            }
            ExprKind::Ident(name) => self.null_test(name, true),
            _ => None,
        }
    }

    fn null_test(&self, name: &str, negated: bool) -> Option<Narrowing> {
        let symbol = self.symbols.lookup(name)?;
        let original = resolve_named_type(&symbol.ty, &self.aliases);
        if !matches!(original, Type::Union(_)) {
            return None;
        }
        let (then_ty, else_ty) = if negated {
            (non_null(&original), Type::Null)
        } else {
            (Type::Null, non_null(&original))
        };
        Some(Narrowing {
            name: name.to_owned(),
            original,
            then_ty,
            else_ty,
        })
    }

    /// Insert a temporarily refined binding into a new scope.
    pub(super) fn enter_narrowed(&mut self, name: &str, ty: &Type) {
        self.symbols.enter_scope();
        if let Some(symbol) = self.symbols.lookup(name).cloned() {
            self.symbols.insert(Symbol {
                name: name.to_owned(),
                kind: SymbolKind::Variable,
                ty: ty.clone(),
                span: symbol.span,
            });
        }
    }

    pub(super) fn leave_narrowed(&mut self) {
        self.symbols.leave_scope();
    }
}
