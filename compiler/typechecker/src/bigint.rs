//! Bigint operator typing, kept beside the checker it extends rather than
//! growing the general expression-inference module (ADR-0016).

use navascript_ast::{BinaryOp, Expr, Span};
use navascript_symbol::Type;

use super::checker::Checker;
use super::types::resolve_named_type;

impl Checker<'_> {
    /// Whether a type resolves to the `bigint` primitive.
    pub(super) fn is_bigint(&self, ty: &Type) -> bool {
        matches!(resolve_named_type(ty, &self.aliases), Type::BigInt)
    }

    /// Type `++`/`--`, which accept numbers and bigints alike.
    pub(super) fn infer_update(&mut self, symbol: &str, operand: &Expr, span: Span) -> Type {
        let current = self.infer_expr(operand);
        let result = if self.is_bigint(&current) {
            Type::BigInt
        } else {
            Type::Number
        };
        if !matches!(
            current,
            Type::Number | Type::BigInt | Type::Unknown | Type::Error
        ) {
            self.error(format!("{symbol} requires a number or bigint"), span);
        }
        self.check_assign_target(operand, &result);
        result
    }

    /// Type a binary operator whose operands involve `bigint`, or return
    /// `None` when it is an ordinary number/string operator.
    pub(super) fn infer_bigint_binary(
        &mut self,
        op: BinaryOp,
        lhs: &Type,
        rhs: &Type,
        span: Span,
    ) -> Option<Type> {
        let lhs_bigint = self.is_bigint(lhs);
        let rhs_bigint = self.is_bigint(rhs);
        if !lhs_bigint && !rhs_bigint {
            return None;
        }
        if !(lhs_bigint && rhs_bigint) {
            self.error("cannot mix bigint with other types", span);
            return Some(Type::Error);
        }
        Some(match op {
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Mod
            | BinaryOp::Pow => Type::BigInt,
            BinaryOp::Eq
            | BinaryOp::StrictEq
            | BinaryOp::NotEq
            | BinaryOp::StrictNotEq
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge => Type::Boolean,
            BinaryOp::And | BinaryOp::Or => {
                self.error("logical operators require booleans", span);
                Type::Error
            }
        })
    }
}
