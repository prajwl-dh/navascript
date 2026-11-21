//! AST → IR lowering. All control flow, scoping and slot allocation decisions
//! are hidden behind `lower`.

use nava_ast::{Expr, ItemKind, Program, Span, TypeAnnotation, TypeExpr};
use nava_diagnostics::{Diagnostic, Diagnostics, LineIndex};
use nava_symbol::Type;
use nava_typechecker::TypedProgram;

use super::builder::FunctionBuilder;
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
}

impl<'a> Lowerer<'a> {
    pub(super) fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message, span, &self.lines));
    }

    pub(super) fn lower_program(&mut self, program: &Program) {
        let mut main_body = Vec::new();
        for item in &program.items {
            match &item.kind {
                ItemKind::Function(decl) => self.lower_function(decl),
                ItemKind::Variable(_) | ItemKind::Expr(_) | ItemKind::Stmt(_) => {
                    main_body.push(item)
                }
                ItemKind::TypeAlias(_) | ItemKind::Interface(_) => {}
            }
        }
        if !main_body.is_empty() {
            self.lower_main(&main_body);
        }
    }

    pub(super) fn lower_main(&mut self, items: &[&nava_ast::Item]) {
        let mut builder = FunctionBuilder::new("main".to_owned(), Vec::new(), IrType::Void);
        for item in items {
            match &item.kind {
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

    pub(super) fn lower_function(&mut self, decl: &nava_ast::FunctionDecl) {
        let params: Vec<IrParam> = decl
            .params
            .iter()
            .map(|param| IrParam {
                name: param.name.name.clone(),
                ty: ir_type(&param.ty),
            })
            .collect();
        let return_ty = if decl.return_ty.is_some() {
            ir_type_opt(&decl.return_ty)
        } else {
            self.typed
                .symbols
                .lookup(&decl.name.name)
                .map(|symbol| match &symbol.ty {
                    Type::Function { ret, .. } => ir_type_from_symbol(ret),
                    _ => IrType::Opaque,
                })
                .unwrap_or(IrType::Opaque)
        };
        let mut builder = FunctionBuilder::new(decl.name.name.clone(), params, return_ty.clone());

        let entry = builder.function.entry;
        for (index, param) in decl.params.iter().enumerate() {
            let ty = ir_type(&param.ty);
            let slot = builder.alloc(ty.clone());
            builder.store(slot, ty, Value::Param(index as u32));
            builder.declare(&param.name, slot);
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
            Some(Type::Function { ret, .. }) => ir_type_from_symbol(ret),
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
            .map(ir_type_from_symbol)
            .unwrap_or(IrType::Opaque)
    }
}

pub(super) fn ir_type(annotation: &Option<TypeAnnotation>) -> IrType {
    ir_type_opt(annotation)
}

pub(super) fn ir_type_opt(annotation: &Option<TypeAnnotation>) -> IrType {
    annotation
        .as_ref()
        .map(|annotation| ir_type_expr(&annotation.expr))
        .unwrap_or(IrType::Opaque)
}

pub(super) fn ir_type_expr(expr: &TypeExpr) -> IrType {
    match expr {
        TypeExpr::Named(name) => match name.as_str() {
            "number" => IrType::Number,
            "string" => IrType::String,
            "boolean" => IrType::Boolean,
            "void" => IrType::Void,
            _ => IrType::Opaque,
        },
        TypeExpr::Array(_) | TypeExpr::Object(_) | TypeExpr::Function { .. } => IrType::Opaque,
    }
}

pub(super) fn ir_type_from_symbol(ty: &Type) -> IrType {
    match ty {
        Type::Number => IrType::Number,
        Type::String => IrType::String,
        Type::Boolean => IrType::Boolean,
        Type::Void => IrType::Void,
        Type::Array(inner) => IrType::Array(Box::new(ir_type_from_symbol(inner))),
        Type::Object(fields) => IrType::Object(
            fields
                .iter()
                .map(|(name, field_ty)| (name.clone(), ir_type_from_symbol(field_ty)))
                .collect(),
        ),
        _ => IrType::Opaque,
    }
}
