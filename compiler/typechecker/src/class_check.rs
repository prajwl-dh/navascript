//! Class member checking and `new`/`super(...)` expression typing.

use nava_ast::{ClassDecl, ClassMember, Expr, Ident, Span};
use nava_symbol::{Symbol, SymbolKind, Type};

use super::checker::Checker;
use super::types::{Subst, is_assignable};

impl Checker<'_> {
    pub(super) fn check_class(&mut self, decl: &ClassDecl) {
        let previous_class = self.current_class.replace(decl.name.name.clone());
        let instance = self.instance_type(&decl.name.name);
        for member in &decl.members {
            match member {
                ClassMember::Constructor { params, body } => {
                    self.symbols.enter_scope();
                    self.insert_self(&decl.name.name, &instance, member_span(member));
                    self.insert_params(params);
                    self.check_block(body, Some(&Type::Void));
                    self.symbols.leave_scope();
                }
                ClassMember::Method {
                    name,
                    params,
                    return_ty,
                    body,
                } => {
                    self.symbols.enter_scope();
                    self.insert_self(&decl.name.name, &instance, name.span);
                    self.insert_params(params);
                    let expected = return_ty
                        .as_ref()
                        .map(|annotation| self.resolve(&annotation.expr, &Subst::new(), name.span))
                        .unwrap_or_else(|| self.infer_return_type(body));
                    self.check_block(body, Some(&expected));
                    self.symbols.leave_scope();
                    if let Some(info) = self.classes.get_mut(&decl.name.name) {
                        if let Some(Type::Function { ret, .. }) = info.methods.get_mut(&name.name) {
                            **ret = expected;
                        }
                    }
                }
                ClassMember::Field { .. } => {}
            }
        }
        self.current_class = previous_class;
    }

    /// Base-constructor parameter types and the function that implements it.
    pub(super) fn super_ctor(&self) -> Option<(Vec<Type>, String)> {
        let class = self.current_class.as_ref()?;
        let base_name = self.classes.get(class)?.base.as_ref()?;
        let info = self.classes.get(base_name)?;
        Some((
            info.ctor_params.clone(),
            info.ctor_owner.clone().unwrap_or_else(|| base_name.clone()),
        ))
    }

    fn insert_self(&mut self, class: &str, instance: &Type, span: Span) {
        self.symbols.insert(Symbol {
            name: "this".to_owned(),
            kind: SymbolKind::Parameter,
            ty: instance.clone(),
            span,
        });
        if let Some(base) = self.classes.get(class).and_then(|info| info.base.clone()) {
            self.symbols.insert(Symbol {
                name: "super".to_owned(),
                kind: SymbolKind::Parameter,
                ty: self.instance_type(&base),
                span,
            });
        }
    }

    fn insert_params(&mut self, params: &[nava_ast::Param]) {
        for param in params {
            let ty = param
                .ty
                .as_ref()
                .map(|annotation| self.resolve(&annotation.expr, &Subst::new(), param.span))
                .unwrap_or(Type::Unknown);
            self.symbols.insert(Symbol {
                name: param.name.name.clone(),
                kind: SymbolKind::Parameter,
                ty,
                span: param.span,
            });
        }
    }

    /// Type-check `new Class(args)` and produce the instance type.
    pub(super) fn infer_new(&mut self, class: &Ident, args: &[Expr], span: Span) -> Type {
        let Some(info) = self.classes.get(&class.name).cloned() else {
            self.error(format!("unknown class '{}'", class.name), class.span);
            return Type::Error;
        };
        let actuals: Vec<Type> = args.iter().map(|arg| self.infer_expr(arg)).collect();
        if info.ctor_params.len() != actuals.len() {
            self.error(
                format!(
                    "expected {} constructor arguments, found {}",
                    info.ctor_params.len(),
                    actuals.len()
                ),
                span,
            );
        }
        for (index, (expected, actual)) in info.ctor_params.iter().zip(&actuals).enumerate() {
            if !is_assignable(expected, actual, &self.aliases) {
                self.error(
                    format!(
                        "argument has type '{}', expected '{}'",
                        actual.display(),
                        expected.display()
                    ),
                    args[index].span,
                );
            }
        }
        Type::ClassInstance {
            name: class.name.clone(),
            fields: info.fields,
        }
    }

    /// Type-check `super(args)` against the base constructor.
    pub(super) fn infer_super_call(&mut self, args: &[Expr], span: Span) -> Type {
        let Some((params, target)) = self.super_ctor() else {
            self.error("'super(...)' requires a base class constructor", span);
            return Type::Error;
        };
        self.super_calls.insert(span, target);
        let actuals: Vec<Type> = args.iter().map(|arg| self.infer_expr(arg)).collect();
        if params.len() != actuals.len() {
            self.error(
                format!(
                    "expected {} base constructor arguments, found {}",
                    params.len(),
                    actuals.len()
                ),
                span,
            );
        }
        for (index, (expected, actual)) in params.iter().zip(&actuals).enumerate() {
            if !is_assignable(expected, actual, &self.aliases) {
                self.error(
                    format!(
                        "argument has type '{}', expected '{}'",
                        actual.display(),
                        expected.display()
                    ),
                    args[index].span,
                );
            }
        }
        Type::Void
    }
}

pub(super) fn member_span(member: &ClassMember) -> Span {
    match member {
        ClassMember::Field { name, .. } => name.span,
        ClassMember::Constructor { params, .. } => params
            .first()
            .map(|param| param.span)
            .unwrap_or(Span::new(nava_ast::FileId(0), 0, 0)),
        ClassMember::Method { name, .. } => name.span,
    }
}
