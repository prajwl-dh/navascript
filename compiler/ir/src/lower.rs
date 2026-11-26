//! AST → IR lowering. All control flow, scoping and slot allocation decisions
//! are hidden behind `lower`.

use std::collections::HashMap;

use navascript_ast::{Expr, FunctionDecl, ItemKind, Program, Span, TypeAnnotation, TypeExpr};
use navascript_diagnostics::{Diagnostic, Diagnostics, LineIndex};
use navascript_symbol::Type;
use navascript_typechecker::TypedProgram;

use super::builder::FunctionBuilder;
use super::closure_names::{captures_in_block, captures_in_items};
use super::ir::{GlobalId, IrGlobal, IrModule, IrParam, IrType, Terminator, Value};

/// The result of lowering: an owned module plus diagnostics in the same shape
/// as every earlier front-end phase.
#[derive(Debug, Clone, PartialEq)]
pub struct LowerOutput {
    pub module: IrModule,
    pub diagnostics: Diagnostics,
}

/// Lower a typed program into platform-independent IR.
///
/// Infallible: unsupported constructs become diagnostics, never panics. The
/// module is verified before it is returned; verifier errors are reported as
/// diagnostics too.
pub fn lower(program: &Program, typed: &TypedProgram, source: &str) -> LowerOutput {
    let mut lowerer = Lowerer {
        module: IrModule::default(),
        diagnostics: Vec::new(),
        lines: LineIndex::new(source),
        typed,
        next_global: 0,
        next_closure: 0,
        program,
        current_subst: HashMap::new(),
        specializations: HashMap::new(),
        function_values: HashMap::new(),
    };
    lowerer.lower_program(program);
    for error in super::verify::verify(&lowerer.module) {
        lowerer.diagnostics.push(Diagnostic::error(
            error.to_string(),
            error.span.unwrap_or(program.span),
            &lowerer.lines,
        ));
    }
    LowerOutput {
        module: lowerer.module,
        diagnostics: Diagnostics::new(lowerer.diagnostics),
    }
}

pub(super) struct Lowerer<'a> {
    pub(super) module: IrModule,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) lines: LineIndex,
    pub(super) typed: &'a TypedProgram,
    pub(super) next_global: GlobalId,
    /// Counter for generated closure functions and function-value thunks.
    pub(super) next_closure: u32,
    pub(super) program: &'a Program,
    /// Type parameters of the generic function currently being specialized.
    pub(super) current_subst: HashMap<String, IrType>,
    /// Monomorphized functions already emitted, keyed by base name + type args.
    pub(super) specializations: HashMap<(String, Vec<IrType>), String>,
    /// Zero-capture thunks already emitted for named functions used as values.
    pub(super) function_values: HashMap<String, String>,
}

impl<'a> Lowerer<'a> {
    pub(super) fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message, span, &self.lines));
    }

    pub(super) fn lower_program(&mut self, program: &navascript_ast::Program) {
        let mut main_body = Vec::new();
        for item in &program.items {
            self.lower_program_item(item, &mut main_body);
        }
        if !main_body.is_empty() {
            self.lower_main(&main_body);
        }
    }

    fn lower_program_item<'p>(
        &mut self,
        item: &'p navascript_ast::Item,
        main_body: &mut Vec<&'p navascript_ast::Item>,
    ) {
        match item.kind.unwrap_export() {
            ItemKind::Function(decl) if decl.type_params.is_empty() => {
                self.lower_function(decl, None)
            }
            ItemKind::Function(_) => {} // emitted on demand as a specialization
            ItemKind::Class(decl) => self.lower_class(decl),
            ItemKind::Variable(_) | ItemKind::Expr(_) | ItemKind::Stmt(_) => main_body.push(item),
            _ => {}
        }
    }

    pub(super) fn lower_main(&mut self, items: &[&navascript_ast::Item]) {
        let mut builder =
            FunctionBuilder::new(super::ENTRY_FUNCTION.to_owned(), Vec::new(), IrType::Void);
        builder.set_captured_names(captures_in_items(items));
        for item in items {
            match item.kind.unwrap_export() {
                ItemKind::Variable(decl) => builder.lower_variable(decl, self),
                ItemKind::Expr(expr) => {
                    builder.lower_expr(expr, self);
                }
                ItemKind::Stmt(stmt) => {
                    if let Some(current) = builder.current {
                        builder.current = builder.lower_stmt(stmt, self, current);
                    }
                }
                _ => {}
            }
        }
        if let Some(current) = builder.current {
            builder.set_terminator(current, Terminator::Return(None));
        }
        self.module.functions.push(builder.finish());
    }

    /// Lower a function. `specialization` supplies the emitted name and the
    /// concrete types for a generic function's parameters; ordinary functions
    /// pass `None`.
    pub(super) fn lower_function(
        &mut self,
        decl: &FunctionDecl,
        specialization: Option<(String, HashMap<String, IrType>)>,
    ) {
        let (name, subst) = match specialization {
            Some((name, subst)) => (name, subst),
            None => (decl.name.name.clone(), HashMap::new()),
        };
        let previous_subst = std::mem::replace(&mut self.current_subst, subst);
        // Use the checker's resolved signature, not the AST annotations: this
        // keeps IR types in step with `Checker::resolve` (including generics).
        let signature = self
            .typed
            .symbols
            .lookup(&decl.name.name)
            .and_then(|symbol| match &symbol.ty {
                Type::Function { params, ret } => Some((params.clone(), (**ret).clone())),
                _ => None,
            });
        let Some((param_types, ret_ty)) = signature else {
            self.error(
                format!("missing checked signature for '{}'", decl.name.name),
                decl.name.span,
            );
            self.current_subst = previous_subst;
            return;
        };
        let params: Vec<IrParam> = decl
            .params
            .iter()
            .zip(&param_types)
            .map(|(param, ty)| IrParam {
                name: param.name.name.clone(),
                ty: ir_type_from_symbol(ty, &self.current_subst),
            })
            .collect();
        let return_ty = ir_type_from_symbol(&ret_ty, &self.current_subst);
        let mut builder = FunctionBuilder::new(name, params, return_ty.clone());
        if decl.is_async {
            if let IrType::Object(fields) = &return_ty {
                if let Some((_, inner)) = fields
                    .iter()
                    .find(|(name, _)| name == super::promises::VALUE_FIELD)
                {
                    builder.async_return = Some(inner.clone());
                }
            }
        }
        builder.set_captured_names(captures_in_block(&decl.body));

        let entry = builder.function.entry;
        for (index, param) in decl.params.iter().enumerate() {
            let ty = ir_type_from_symbol(&param_types[index], &self.current_subst);
            if builder.captured_names.contains(&param.name.name) {
                builder.bind_cell(&param.name, Value::Param(index as u32), ty);
            } else {
                let slot = builder.alloc(ty.clone());
                builder.store(slot, ty, Value::Param(index as u32));
                builder.declare(&param.name, slot);
            }
        }

        let end = builder.lower_block(&decl.body, self, entry);
        if let Some(block) = end {
            let value = if return_ty == IrType::Void {
                None
            } else {
                self.error(
                    format!(
                        "function '{}' may fall through without returning",
                        decl.name.name
                    ),
                    decl.body.span,
                );
                Some(Value::Null)
            };
            builder.set_terminator(block, Terminator::Return(value));
        }
        self.module.functions.push(builder.finish());
        self.current_subst = previous_subst;
    }

    /// Allocate a distinct name for a generated closure or function-value thunk.
    pub(super) fn fresh_closure_name(&mut self, prefix: &str) -> String {
        let name = format!("{prefix}{}", self.next_closure);
        self.next_closure += 1;
        name
    }

    /// The checked result type of a call expression, used for both direct and
    /// indirect calls.
    pub(super) fn call_result_type(&self, span: Span) -> IrType {
        self.typed
            .types
            .get(&span)
            .map(|ty| ir_type_from_symbol(ty, &self.current_subst))
            .unwrap_or(IrType::Opaque)
    }

    pub(super) fn string_global(&mut self, value: &str) -> GlobalId {
        let id = self.next_global;
        self.next_global += 1;
        self.module.globals.push(IrGlobal {
            id,
            value: value.to_owned(),
        });
        id
    }

    pub(super) fn return_type_of(&self, name: &str) -> IrType {
        match self.typed.symbols.lookup(name).map(|symbol| &symbol.ty) {
            Some(Type::Function { ret, .. }) => ir_type_from_symbol(ret, &self.current_subst),
            Some(Type::Void) => IrType::Void,
            _ => IrType::Opaque,
        }
    }

    pub(super) fn is_known_builtin(name: &str) -> bool {
        matches!(name, "console.log")
    }

    pub(super) fn expr_ty(&self, expr: &Expr) -> IrType {
        self.typed
            .types
            .get(&expr.span)
            .map(|ty| ir_type_from_symbol(ty, &self.current_subst))
            .unwrap_or(IrType::Opaque)
    }
}

pub(super) fn ir_type(annotation: &Option<TypeAnnotation>) -> IrType {
    annotation
        .as_ref()
        .map(|annotation| ir_type_expr(&annotation.expr))
        .unwrap_or(IrType::Opaque)
}

pub(super) fn ir_type_expr(expr: &TypeExpr) -> IrType {
    match expr {
        TypeExpr::Named(name) => match name.as_str() {
            "number" => IrType::Number,
            "bigint" => IrType::BigInt,
            "string" => IrType::String,
            "boolean" => IrType::Boolean,
            "void" => IrType::Void,
            _ => IrType::Opaque,
        },
        TypeExpr::Union(members) => members
            .iter()
            .filter(|member| !matches!(member, TypeExpr::Named(name) if name == "null"))
            .find_map(|member| {
                let ty = ir_type_expr(member);
                (ty != IrType::Opaque).then_some(ty)
            })
            .unwrap_or(IrType::Opaque),
        TypeExpr::Intersection(members) => {
            members.first().map(ir_type_expr).unwrap_or(IrType::Opaque)
        }
        TypeExpr::Function { params, ret } => IrType::Function {
            params: params.iter().map(ir_type_expr).collect(),
            ret: Box::new(ir_type_expr(ret)),
        },
        TypeExpr::Generic { .. } | TypeExpr::Array(_) | TypeExpr::Object(_) => IrType::Opaque,
    }
}

pub(super) fn ir_type_from_symbol(ty: &Type, subst: &HashMap<String, IrType>) -> IrType {
    match ty {
        Type::TypeParam(name) => subst.get(name).cloned().unwrap_or(IrType::Opaque),
        Type::Number => IrType::Number,
        Type::BigInt => IrType::BigInt,
        Type::String => IrType::String,
        Type::Boolean => IrType::Boolean,
        Type::Void => IrType::Void,
        Type::ClassInstance { fields, .. } => IrType::Object(
            fields
                .iter()
                .map(|(name, field_ty)| (name.clone(), ir_type_from_symbol(field_ty, subst)))
                .collect(),
        ),
        Type::Union(members) => members
            .iter()
            .filter(|member| !matches!(member, Type::Null))
            .find_map(|member| {
                let ty = ir_type_from_symbol(member, subst);
                (ty != IrType::Opaque).then_some(ty)
            })
            .unwrap_or(IrType::Opaque),
        Type::Intersection(members) => {
            let mut fields: Vec<(String, IrType)> = Vec::new();
            for member in members {
                match ir_type_from_symbol(member, subst) {
                    IrType::Object(mut own) => {
                        for (name, ty) in own.drain(..) {
                            if let Some(existing) =
                                fields.iter_mut().find(|(field, _)| field == &name)
                            {
                                existing.1 = ty;
                            } else {
                                fields.push((name, ty));
                            }
                        }
                    }
                    other if other != IrType::Opaque => return other,
                    _ => {}
                }
            }
            if fields.is_empty() {
                IrType::Opaque
            } else {
                // Object literals lay fields out in sorted order; intersections
                // must use the same order so member indices agree.
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                IrType::Object(fields)
            }
        }
        Type::Function { params, ret } => IrType::Function {
            params: params
                .iter()
                .map(|param| ir_type_from_symbol(param, subst))
                .collect(),
            ret: Box::new(ir_type_from_symbol(ret, subst)),
        },
        Type::Promise(inner) => super::promises::promise_type(ir_type_from_symbol(inner, subst)),
        Type::Array(inner) => IrType::Array(Box::new(ir_type_from_symbol(inner, subst))),
        Type::Object(fields) => IrType::Object(
            fields
                .iter()
                .map(|(name, field_ty)| (name.clone(), ir_type_from_symbol(field_ty, subst)))
                .collect(),
        ),
        _ => IrType::Opaque,
    }
}
