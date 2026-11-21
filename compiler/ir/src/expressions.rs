//! Expression lowering into IR instructions.

use nava_ast::{BinaryOp as AstBinaryOp, Expr, ExprKind, UnaryOp as AstUnaryOp};

use super::builder::FunctionBuilder;
use super::ir::{Instruction, IrType, UnaryOp, Value};
use super::lower::Lowerer;

impl FunctionBuilder {
    pub(super) fn lower_expr(&mut self, expr: &Expr, lowerer: &mut Lowerer) -> Value {
        match &expr.kind {
            ExprKind::Number(value) => Value::Number(*value),
            ExprKind::Bool(value) => Value::Bool(*value),
            ExprKind::Null => Value::Null,
            ExprKind::String(value) => {
                let global = lowerer.string_global(value);
                Value::String(global)
            }
            ExprKind::Ident(name) => match self.lookup(name) {
                Some((slot, ty)) => self.load(slot, ty),
                None => {
                    lowerer.error(format!("unknown value '{name}' in IR"), expr.span);
                    Value::Number(0.0)
                }
            },
            ExprKind::Binary { op, left, right } => {
                let lhs_ty = lowerer.expr_ty(left);
                let rhs_ty = lowerer.expr_ty(right);
                let lhs = self.lower_expr(left, lowerer);
                let rhs = self.lower_expr(right, lowerer);
                let result = self.fresh_value();
                if *op == AstBinaryOp::Add {
                    self.emit_add(lhs, rhs, lhs_ty, rhs_ty, result);
                } else if lhs_ty == IrType::String || rhs_ty == IrType::String {
                    // Equality on strings needs `ptr` operands.
                    let lhs = if lhs_ty == IrType::String {
                        lhs
                    } else {
                        self.number_to_string(lhs)
                    };
                    let rhs = if rhs_ty == IrType::String {
                        rhs
                    } else {
                        self.number_to_string(rhs)
                    };
                    match op {
                        AstBinaryOp::Eq
                        | AstBinaryOp::StrictEq
                        | AstBinaryOp::NotEq
                        | AstBinaryOp::StrictNotEq => {
                            self.emit(Instruction::StringEquals {
                                result,
                                left: lhs,
                                right: rhs,
                            });
                            if matches!(op, AstBinaryOp::NotEq | AstBinaryOp::StrictNotEq) {
                                let negated = self.fresh_value();
                                self.emit(Instruction::Unary {
                                    result: negated,
                                    op: UnaryOp::Not,
                                    operand: Value::Local(result),
                                });
                                return Value::Local(negated);
                            }
                        }
                        _ => {
                            lowerer.error("unsupported string operator", expr.span);
                        }
                    }
                } else {
                    self.emit(Instruction::Binary {
                        result,
                        op: super::operators::to_ir_binary(*op),
                        lhs,
                        rhs,
                    });
                }
                Value::Local(result)
            }
            ExprKind::Unary { op, operand } => {
                let operand = self.lower_expr(operand, lowerer);
                match op {
                    AstUnaryOp::Pos => operand,
                    AstUnaryOp::Neg | AstUnaryOp::Not => {
                        let result = self.fresh_value();
                        self.emit(Instruction::Unary {
                            result,
                            op: match op {
                                AstUnaryOp::Neg => UnaryOp::Neg,
                                _ => UnaryOp::Not,
                            },
                            operand,
                        });
                        Value::Local(result)
                    }
                }
            }
            ExprKind::Assign { target, value } => {
                let value = self.lower_expr(value, lowerer);
                self.lower_assign(target, &value, lowerer);
                value
            }
            ExprKind::CompoundAssign { op, target, value } => {
                self.lower_compound_assign(*op, target, value, lowerer)
            }
            ExprKind::Update {
                op,
                operand,
                prefix,
            } => self.lower_update(*op, operand, *prefix, lowerer),
            ExprKind::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                let ty = lowerer.expr_ty(expr);
                self.lower_ternary(condition, then_expr, else_expr, ty, lowerer)
            }
            ExprKind::Arrow { .. } => {
                lowerer.error(
                    "arrow functions are only supported as variable initialisers",
                    expr.span,
                );
                Value::Null
            }
            ExprKind::Call { callee, args } => self.lower_call(callee, args, lowerer),
            ExprKind::Index {
                object,
                index,
                optional,
            } => {
                let object_ty = lowerer.expr_ty(object);
                if *optional {
                    return self.lower_optional_access(
                        super::operators::OptionalAccess {
                            object,
                            target: super::operators::OptionalTarget::Index(index),
                            object_ty,
                            result_ty: lowerer.expr_ty(expr),
                            span: expr.span,
                        },
                        lowerer,
                    );
                }
                let object = self.lower_expr(object, lowerer);
                let index = self.lower_expr(index, lowerer);
                let result = self.fresh_value();
                match object_ty {
                    IrType::Array(element_ty) => {
                        self.emit(Instruction::ArrayGet {
                            result,
                            array: object,
                            index,
                            element_ty: *element_ty,
                        });
                    }
                    IrType::String => {
                        self.emit(Instruction::StringCharAt {
                            result,
                            value: object,
                            index,
                        });
                    }
                    _ => {
                        lowerer.error("indexing requires an array or string", expr.span);
                    }
                }
                Value::Local(result)
            }
            ExprKind::Member {
                object,
                property,
                optional,
            } => {
                let object_ty = lowerer.expr_ty(object);
                if *optional {
                    return self.lower_optional_access(
                        super::operators::OptionalAccess {
                            object,
                            target: super::operators::OptionalTarget::Member(property),
                            object_ty,
                            result_ty: lowerer.expr_ty(expr),
                            span: expr.span,
                        },
                        lowerer,
                    );
                }
                let object = self.lower_expr(object, lowerer);
                let result = self.fresh_value();
                match object_ty {
                    IrType::String if property == "length" => {
                        self.emit(Instruction::StringLength {
                            result,
                            value: object,
                        });
                    }
                    IrType::Array(_) if property == "length" => {
                        self.emit(Instruction::ArrayLength {
                            result,
                            array: object,
                        });
                    }
                    IrType::Object(fields) => {
                        if let Some((field_index, (_, field_ty))) = fields
                            .iter()
                            .enumerate()
                            .find(|(_, (name, _))| name == property)
                        {
                            self.emit(Instruction::ObjectGet {
                                result,
                                object,
                                field_index: field_index as u32,
                                field_ty: field_ty.clone(),
                            });
                        } else {
                            lowerer.error(format!("unknown field '{property}'"), expr.span);
                        }
                    }
                    _ => lowerer.error("member access requires an object", expr.span),
                }
                Value::Local(result)
            }
            ExprKind::ArrayLit(items) => {
                let element_ty = match lowerer.expr_ty(expr) {
                    IrType::Array(inner) => *inner,
                    _ => IrType::Opaque,
                };
                let array = self.fresh_value();
                self.emit(Instruction::ArrayNew {
                    result: array,
                    element_ty: element_ty.clone(),
                    length: items.len() as u32,
                });
                for (index, item) in items.iter().enumerate() {
                    let value = self.lower_expr(item, lowerer);
                    self.emit(Instruction::ArraySet {
                        array: Value::Local(array),
                        index: Value::Number(index as f64),
                        value,
                        element_ty: element_ty.clone(),
                    });
                }
                Value::Local(array)
            }
            ExprKind::ObjectLit(fields) => {
                let field_types = match lowerer.expr_ty(expr) {
                    IrType::Object(fields) => fields,
                    _ => Vec::new(),
                };
                let object = self.fresh_value();
                self.emit(Instruction::ObjectNew {
                    result: object,
                    fields: field_types.clone(),
                });
                // Field order comes from the inferred object type (a sorted
                // map), not from source order; find each field's real index.
                for field in fields {
                    let value = self.lower_expr(&field.value, lowerer);
                    if let Some((index, (_, field_ty))) = field_types
                        .iter()
                        .enumerate()
                        .find(|(_, (name, _))| name == &field.name)
                    {
                        self.emit(Instruction::ObjectSet {
                            object: Value::Local(object),
                            field_index: index as u32,
                            value,
                            field_ty: field_ty.clone(),
                        });
                    } else {
                        lowerer.error(format!("unknown field '{}'", field.name), field.span);
                    }
                }
                Value::Local(object)
            }
            ExprKind::Paren(inner) => self.lower_expr(inner, lowerer),
        }
    }
}
