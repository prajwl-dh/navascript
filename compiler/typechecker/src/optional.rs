//! Member/index inference and optional-chaining validation.

use nava_ast::{Expr, Span};
use nava_symbol::Type;

use super::checker::Checker;
use super::types::{object_field, resolve_named_type};

impl Checker<'_> {
    pub(super) fn infer_index(
        &mut self,
        object: &Expr,
        index: &Expr,
        optional: bool,
        span: Span,
    ) -> Type {
        let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
        let index_ty = self.infer_expr(index);
        if !matches!(index_ty, Type::Number | Type::Unknown | Type::Error) {
            self.error("array index must be a number", index.span);
        }
        let ty = match object_ty {
            Type::Array(inner) => *inner,
            Type::String => Type::String,
            Type::Error | Type::Unknown => Type::Unknown,
            other => {
                self.error(
                    format!("cannot index a value of type '{}'", other.display()),
                    span,
                );
                Type::Error
            }
        };
        self.validate_optional_result(&ty, optional, span);
        ty
    }

    pub(super) fn infer_member(
        &mut self,
        object: &Expr,
        property: &str,
        optional: bool,
        span: Span,
    ) -> Type {
        let object_ty = resolve_named_type(&self.infer_expr(object), &self.aliases);
        let ty = if let Some(found) = object_field(&object_ty, property, &self.aliases) {
            found
        } else {
            match object_ty {
                Type::Object(_) | Type::ClassInstance { .. } | Type::Intersection(_) => {
                    self.error(format!("property '{property}' does not exist"), span);
                    Type::Error
                }
                Type::String | Type::Array(_) if property == "length" => Type::Number,
                Type::Error | Type::Unknown => Type::Unknown,
                other => {
                    self.error(
                        format!(
                            "property access requires an object, found '{}'",
                            other.display()
                        ),
                        span,
                    );
                    Type::Error
                }
            }
        };
        self.validate_optional_result(&ty, optional, span);
        ty
    }

    pub(super) fn validate_optional_result(&mut self, ty: &Type, optional: bool, span: Span) {
        if optional
            && !matches!(
                ty,
                Type::String | Type::Array(_) | Type::Object(_) | Type::Unknown | Type::Error
            )
        {
            self.error("optional chaining requires a reference-typed result", span);
        }
    }

    pub(super) fn reject_optional_target(&mut self, optional: bool, span: Span) -> bool {
        if optional {
            self.error("cannot assign through optional chaining", span);
            true
        } else {
            false
        }
    }
}
