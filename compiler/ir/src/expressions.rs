//! Expression lowering into IR instructions.

use nava_ast::{BinaryOp as AstBinaryOp, Expr, ExprKind, UnaryOp as AstUnaryOp};

use super::builder::FunctionBuilder;
use super::ir::{BinaryOp, Instruction, IrType, UnaryOp, Value};
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
                if lhs_ty == IrType::String || rhs_ty == IrType::String {
                    // Coerce a numeric operand to a string so both runtime
                    // calls receive `ptr` values (JavaScript-style `+`).
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
                        AstBinaryOp::Add => {
                            self.emit(Instruction::StringConcat {
                                result,
                                left: lhs,
                                right: rhs,
                            });
                        }
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
                        op: binary_op(*op),
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
            ExprKind::Call { callee, args } => {
                let callee_name = match &callee.kind {
                    ExprKind::Ident(name) => name.clone(),
                    ExprKind::Member { object, property } => match &object.kind {
                        ExprKind::Ident(object) => format!("{object}.{property}"),
                        _ => {
                            lowerer.error("unsupported call target in IR", callee.span);
                            "unknown".to_owned()
                        }
                    },
                    _ => {
                        lowerer.error("unsupported call target in IR", callee.span);
                        "unknown".to_owned()
                    }
                };
                let args: Vec<Value> = args
                    .iter()
                    .map(|arg| self.lower_expr(arg, lowerer))
                    .collect();
                if (callee_name.contains('.') || callee_name == "unknown")
                    && !Lowerer::is_known_builtin(&callee_name)
                {
                    lowerer.error(
                        format!("unsupported call target '{callee_name}' in IR"),
                        callee.span,
                    );
                }
                let result_ty = if Lowerer::is_known_builtin(&callee_name) {
                    IrType::Void
                } else {
                    lowerer.return_type_of(&callee_name)
                };
                let result = match result_ty {
                    IrType::Void => None,
                    _ => Some(self.fresh_value()),
                };
                self.emit(Instruction::Call {
                    result,
                    result_ty,
                    callee: callee_name,
                    args,
                });
                result.map_or(Value::Null, Value::Local)
            }
            ExprKind::Index { object, index } => {
                let object_ty = lowerer.expr_ty(object);
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
            ExprKind::Member { object, property } => {
                let object_ty = lowerer.expr_ty(object);
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

fn binary_op(op: AstBinaryOp) -> BinaryOp {
    match op {
        AstBinaryOp::Add => BinaryOp::Add,
        AstBinaryOp::Sub => BinaryOp::Sub,
        AstBinaryOp::Mul => BinaryOp::Mul,
        AstBinaryOp::Div => BinaryOp::Div,
        AstBinaryOp::Mod => BinaryOp::Mod,
        AstBinaryOp::Eq => BinaryOp::Eq,
        AstBinaryOp::StrictEq => BinaryOp::Eq,
        AstBinaryOp::NotEq => BinaryOp::Ne,
        AstBinaryOp::StrictNotEq => BinaryOp::Ne,
        AstBinaryOp::Lt => BinaryOp::Lt,
        AstBinaryOp::Le => BinaryOp::Le,
        AstBinaryOp::Gt => BinaryOp::Gt,
        AstBinaryOp::Ge => BinaryOp::Ge,
        AstBinaryOp::And => BinaryOp::And,
        AstBinaryOp::Or => BinaryOp::Or,
    }
}
