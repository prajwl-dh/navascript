//! Expression type inference and assignment validation.

use nava_ast::{BinaryOp, Expr, ExprKind, Span, UnaryOp};
use nava_symbol::Type;
use std::collections::BTreeMap;

use super::checker::Checker;
use super::types::{is_assignable, object_field, resolve_named_type};

impl<'a> Checker<'a> {
    pub(super) fn infer_expr(&mut self, expr: &Expr) -> Type {
        let ty = match &expr.kind {
            ExprKind::Number(_) => Type::Number,
            ExprKind::BigInt(_) => Type::BigInt,
            ExprKind::String(_) => Type::String,
            ExprKind::Bool(_) => Type::Boolean,
            ExprKind::Null => Type::Null,
            ExprKind::This | ExprKind::Super => {
                let name = if matches!(expr.kind, ExprKind::This) {
                    "this"
                } else {
                    "super"
                };
                match self.symbols.lookup(name) {
                    Some(symbol) => symbol.ty.clone(),
                    None => {
                        self.error(format!("'{name}' is only valid inside a class"), expr.span);
                        Type::Error
                    }
                }
            }
            ExprKind::New { class, args } => self.infer_new(class, args, expr.span),
            ExprKind::SuperCall { args } => self.infer_super_call(args, expr.span),
            ExprKind::Ident(name) => match self.symbols.lookup(name) {
                Some(symbol) => symbol.ty.clone(),
                None => {
                    self.error(format!("unknown identifier '{name}'"), expr.span);
                    Type::Error
                }
            },
            ExprKind::Binary { op, left, right } => {
                let lhs = self.infer_expr(left);
                let narrowing = self.narrow_binding(left);
                if let Some(narrowing) = &narrowing {
                    let ty = if *op == BinaryOp::And {
                        &narrowing.then_ty
                    } else {
                        &narrowing.else_ty
                    };
                    if matches!(op, BinaryOp::And | BinaryOp::Or) {
                        self.enter_narrowed(&narrowing.name, ty);
                    }
                }
                let rhs = self.infer_expr(right);
                if narrowing.is_some() && matches!(op, BinaryOp::And | BinaryOp::Or) {
                    self.leave_narrowed();
                }
                self.infer_binary(*op, &lhs, &rhs, expr.span)
            }
            ExprKind::Unary { op, operand } => {
                let ty = self.infer_expr(operand);
                match op {
                    UnaryOp::Not => Type::Boolean,
                    UnaryOp::Neg | UnaryOp::Pos if self.is_bigint(&ty) => Type::BigInt,
                    UnaryOp::Neg | UnaryOp::Pos if matches!(ty, Type::Number | Type::Unknown) => {
                        Type::Number
                    }
                    UnaryOp::Neg | UnaryOp::Pos => {
                        self.error("unary operator expects a number or bigint", expr.span);
                        Type::Error
                    }
                }
            }
            ExprKind::Assign { target, value } => {
                let actual = self.infer_expr(value);
                self.check_assign_target(target, &actual);
                actual
            }
            ExprKind::CompoundAssign { op, target, value } => {
                let current = self.infer_expr(target);
                let operand = self.infer_expr(value);
                let result = self.infer_binary(op.binary_op(), &current, &operand, expr.span);
                self.check_assign_target(target, &result);
                result
            }
            ExprKind::Update { op, operand, .. } => {
                self.infer_update(op.symbol(), operand, expr.span)
            }
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.infer_expr(condition);
                let then_ty = self.infer_expr(then_expr);
                let else_ty = self.infer_expr(else_expr);
                if then_ty != else_ty
                    && !matches!(then_ty, Type::Unknown | Type::Error)
                    && !matches!(else_ty, Type::Unknown | Type::Error)
                {
                    self.error(
                        format!(
                            "ternary branches have different types '{}' and '{}'",
                            then_ty.display(),
                            else_ty.display()
                        ),
                        expr.span,
                    );
                    Type::Error
                } else if matches!(then_ty, Type::Unknown | Type::Error) {
                    else_ty
                } else {
                    then_ty
                }
            }
            ExprKind::Arrow {
                params,
                return_ty,
                body,
            } => self.infer_arrow(params, return_ty, body),
            ExprKind::Call {
                callee,
                args,
                type_args,
                ..
            } => {
                let (name, callee_ty, method_target) = match &callee.kind {
                    ExprKind::Ident(name) => (Some(name.clone()), self.infer_expr(callee), None),
                    ExprKind::Member {
                        object,
                        property,
                        optional: false,
                    } => {
                        let object_ty = self.infer_expr(object);
                        let target = self
                            .class_of(&object_ty)
                            .and_then(|class| self.lookup_method(&class, property));
                        match target {
                            Some((mangled, fun_ty)) => (None, fun_ty, Some(mangled)),
                            None => (None, self.infer_expr(callee), None),
                        }
                    }
                    _ => (None, self.infer_expr(callee), None),
                };
                let result =
                    self.infer_call(name.as_deref(), &callee_ty, args, type_args, expr.span);
                if let Some(mangled) = method_target {
                    self.method_calls.insert(expr.span, mangled);
                }
                result
            }
            ExprKind::Index {
                object,
                index,
                optional,
            } => self.infer_index(object, index, *optional, expr.span),
            ExprKind::Member {
                object,
                property,
                optional,
            } => self.infer_member(object, property, *optional, expr.span),
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
        self.types.insert(expr.span, ty.clone());
        ty
    }

    /// Check identifier, property and element assignment targets alike.
    pub(super) fn check_assign_target(&mut self, target: &Expr, actual: &Type) {
        match &target.kind {
            ExprKind::Ident(name) => {
                match self.symbols.lookup(name).map(|symbol| symbol.ty.clone()) {
                    Some(expected) => self.check_assignment(&expected, actual, target.span),
                    None => self.error(format!("unknown identifier '{name}'"), target.span),
                }
            }
            ExprKind::Member {
                object,
                property,
                optional,
            } => {
                if self.reject_optional_target(*optional, target.span) {
                    return;
                }
                let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
                if let Some(expected) = object_field(&object_ty, property, &self.aliases) {
                    self.check_assignment(&expected, actual, target.span);
                    return;
                }
                match object_ty {
                    Type::Object(_) | Type::ClassInstance { .. } | Type::Intersection(_) => {
                        self.error(format!("property '{property}' does not exist"), target.span)
                    }
                    Type::Error | Type::Unknown => {}
                    other => self.error(
                        format!("assignment requires an object, found '{}'", other.display()),
                        target.span,
                    ),
                }
            }
            ExprKind::Index {
                object,
                index,
                optional,
            } => {
                if self.reject_optional_target(*optional, target.span) {
                    return;
                }
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

    pub(super) fn check_assignment(&mut self, expected: &Type, actual: &Type, span: Span) {
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

    pub(super) fn infer_binary(
        &mut self,
        op: BinaryOp,
        lhs: &Type,
        rhs: &Type,
        span: Span,
    ) -> Type {
        if let Some(ty) = self.infer_bigint_binary(op, lhs, rhs, span) {
            return ty;
        }
        match op {
            BinaryOp::Add if matches!(lhs, Type::String) || matches!(rhs, Type::String) => {
                Type::String
            }
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow => {
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
}
