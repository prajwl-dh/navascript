//! Class lowering: methods become named functions with `this` as their first
//! parameter. Dispatch stays static — no vtables are needed yet.

use navascript_ast::{Block, ClassDecl, ClassMember, Expr, Ident, Param};
use navascript_symbol::Type;

use super::builder::FunctionBuilder;
use super::ir::{Instruction, IrParam, IrType, Terminator, Value};
use super::lower::{Lowerer, ir_type, ir_type_from_symbol};

impl Lowerer<'_> {
    /// Emit one IR function per constructor and method of a class.
    pub(super) fn lower_class(&mut self, decl: &ClassDecl) {
        let Some(info) = self.typed.classes.get(&decl.name.name).cloned() else {
            self.error(
                format!("missing checked info for class '{}'", decl.name.name),
                decl.name.span,
            );
            return;
        };
        let object_ty = IrType::Object(
            info.fields
                .iter()
                .map(|(name, ty)| (name.clone(), ir_type_from_symbol(ty, &self.current_subst)))
                .collect(),
        );
        for member in &decl.members {
            match member {
                ClassMember::Field { .. } => {}
                ClassMember::Constructor { params, body } => {
                    self.lower_class_function(
                        &format!("{}__ctor", decl.name.name),
                        object_ty.clone(),
                        params,
                        IrType::Void,
                        body,
                    );
                }
                ClassMember::Method {
                    name,
                    params,
                    return_ty,
                    body,
                } => {
                    let ret_ty = info
                        .methods
                        .get(&name.name)
                        .and_then(|ty| match ty {
                            Type::Function { ret, .. } => {
                                Some(ir_type_from_symbol(ret, &self.current_subst))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| {
                            return_ty
                                .as_ref()
                                .map(|annotation| ir_type(&Some(annotation.clone())))
                                .unwrap_or(IrType::Opaque)
                        });
                    self.lower_class_function(
                        &format!("{}_{}", decl.name.name, name.name),
                        object_ty.clone(),
                        params,
                        ret_ty,
                        body,
                    );
                }
            }
        }
    }

    fn lower_class_function(
        &mut self,
        name: &str,
        object_ty: IrType,
        params: &[Param],
        return_ty: IrType,
        body: &Block,
    ) {
        let mut ir_params = vec![IrParam {
            name: "this".to_owned(),
            ty: object_ty.clone(),
        }];
        for param in params {
            ir_params.push(IrParam {
                name: param.name.name.clone(),
                ty: ir_type(&param.ty),
            });
        }
        let this_param_ty = ir_params[0].ty.clone();
        let mut builder = FunctionBuilder::new(name.to_owned(), ir_params, return_ty.clone());
        let this_slot = builder.alloc(object_ty.clone());
        builder.store(this_slot, object_ty, Value::Param(0));
        builder.this_slot = Some(this_slot);
        builder.this_ty = Some(this_param_ty);
        for (index, param) in params.iter().enumerate() {
            let ty = ir_type(&param.ty);
            let slot = builder.alloc(ty.clone());
            builder.store(slot, ty, Value::Param((index + 1) as u32));
            builder.declare(&param.name, slot);
        }
        let entry = builder.function.entry;
        if let Some(end) = builder.lower_block(body, self, entry) {
            let value = if return_ty == IrType::Void {
                None
            } else {
                self.error(
                    format!("function '{name}' may fall through without returning"),
                    body.span,
                );
                Some(Value::Null)
            };
            builder.set_terminator(end, Terminator::Return(value));
        }
        self.module.functions.push(builder.finish());
    }
}

impl FunctionBuilder {
    /// Allocate an instance and invoke its constructor, if it has one.
    pub(super) fn lower_new(
        &mut self,
        class: &Ident,
        args: &[Expr],
        lowerer: &mut Lowerer,
    ) -> Value {
        let Some(info) = lowerer.typed.classes.get(&class.name).cloned() else {
            lowerer.error(format!("unknown class '{}'", class.name), class.span);
            return Value::Null;
        };
        let fields: Vec<(String, IrType)> = info
            .fields
            .iter()
            .map(|(name, ty)| {
                (
                    name.clone(),
                    ir_type_from_symbol(ty, &lowerer.current_subst),
                )
            })
            .collect();
        let object = self.fresh_value();
        self.emit(Instruction::ObjectNew {
            result: object,
            fields,
        });
        if info.has_ctor {
            let mut call_args = vec![Value::Local(object)];
            for arg in args {
                call_args.push(self.lower_expr(arg, lowerer));
            }
            self.emit(Instruction::Call {
                result: None,
                result_ty: IrType::Void,
                callee: format!(
                    "{}__ctor",
                    info.ctor_owner.as_deref().unwrap_or(&class.name)
                ),
                args: call_args,
            });
        }
        Value::Local(object)
    }

    /// Emit a `super(args)` constructor call on the current `this`.
    pub(super) fn lower_super_call(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        lowerer: &mut Lowerer,
    ) -> Value {
        let Some(target) = lowerer.typed.super_calls.get(&expr.span).cloned() else {
            lowerer.error("missing base constructor for 'super(...)'", expr.span);
            return Value::Null;
        };
        let mut call_args = vec![self.lower_self(expr, lowerer)];
        for arg in args {
            call_args.push(self.lower_expr(arg, lowerer));
        }
        self.emit(Instruction::Call {
            result: None,
            result_ty: IrType::Void,
            callee: format!("{target}__ctor"),
            args: call_args,
        });
        Value::Null
    }

    /// Load `this` (or `super`, which shares the same slot).
    pub(super) fn lower_self(&mut self, expr: &Expr, lowerer: &mut Lowerer) -> Value {
        match (self.this_slot, self.this_ty.clone()) {
            (Some(slot), Some(ty)) => self.load(slot, ty),
            _ => {
                lowerer.error("'this' is only valid inside a class", expr.span);
                Value::Null
            }
        }
    }
}
