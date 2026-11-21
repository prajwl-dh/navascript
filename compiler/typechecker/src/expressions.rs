//! Expression type inference and assignment validation.

use nava_ast::{BinaryOp, Expr, ExprKind, Span, UnaryOp};
use nava_symbol::Type;
use std::collections::BTreeMap;

use super::checker::Checker;
use super::types::{is_assignable, resolve_named_type};

impl<'a> Checker<'a> {
    pub(super) fn infer_expr(&mut self, expr: &Expr) -> Type {
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
            ExprKind::CompoundAssign { op, target, value } => {
                let current = self.infer_expr(target);
                let operand = self.infer_expr(value);
                let result = self.infer_binary(op.binary_op(), &current, &operand, expr.span);
                self.check_assign_target(target, &result);
                result
            }
            ExprKind::Update { op, operand, .. } => {
                let current = self.infer_expr(operand);
                if !matches!(current, Type::Number | Type::Unknown | Type::Error) {
                    self.error(format!("{} requires a number", op.symbol()), expr.span);
                }
                self.check_assign_target(operand, &Type::Number);
                Type::Number
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
                    Type::String => Type::String,
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
                    Type::String | Type::Array(_) if property == "length" => Type::Number,
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

    pub(super) fn infer_call(&mut self, callee: &Type, args: &[Expr], span: Span) -> Type {
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
