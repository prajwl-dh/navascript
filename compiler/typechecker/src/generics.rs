//! Generic type resolution, inference and call instantiation.

use nava_ast::{Expr, Ident, Span, TypeExpr};
use nava_symbol::Type;

use super::checker::Checker;
use super::types::{AliasTable, Subst, is_assignable, resolve_named_type, substitute};

impl Checker<'_> {
    /// Resolve a type expression, applying both declaration-level substitutions
    /// (a generic function's body) and generic type references (`Box<T>`).
    pub(super) fn resolve(&mut self, expr: &TypeExpr, subst: &Subst, span: Span) -> Type {
        match expr {
            TypeExpr::Named(name) => {
                if let Some(ty) = subst.get(name) {
                    return ty.clone();
                }
                match name.as_str() {
                    "number" => Type::Number,
                    "bigint" => Type::BigInt,
                    "string" => Type::String,
                    "boolean" => Type::Boolean,
                    "void" => Type::Void,
                    "null" => Type::Null,
                    other => {
                        if let Some(ty) = self.aliases.get(other) {
                            substitute(ty, subst)
                        } else if self.classes.contains_key(other) {
                            self.instance_type(other)
                        } else {
                            Type::Named(other.to_owned())
                        }
                    }
                }
            }
            TypeExpr::Generic { name, args } => {
                let mut resolved = Vec::new();
                for arg in args {
                    resolved.push(self.resolve(arg, subst, span));
                }
                let (Some(params), Some(body)) = (
                    self.generic_params.get(name).cloned(),
                    self.aliases.get(name).cloned(),
                ) else {
                    self.error(format!("unknown generic type '{name}'"), span);
                    return Type::Error;
                };
                if params.len() != resolved.len() {
                    self.error(
                        format!(
                            "expected {} type arguments for '{}', found {}",
                            params.len(),
                            name,
                            resolved.len()
                        ),
                        span,
                    );
                    return Type::Error;
                }
                let local: Subst = params.into_iter().zip(resolved).collect();
                substitute(&substitute(&body, subst), &local)
            }
            TypeExpr::Array(inner) => Type::Array(Box::new(self.resolve(inner, subst, span))),
            TypeExpr::Object(fields) => {
                let mut resolved = std::collections::BTreeMap::new();
                for field in fields {
                    resolved.insert(
                        field.name.clone(),
                        self.resolve(&field.ty, subst, field.span),
                    );
                }
                Type::Object(resolved)
            }
            TypeExpr::Function { params, ret } => {
                let mut resolved_params = Vec::new();
                for param in params {
                    resolved_params.push(self.resolve(param, subst, span));
                }
                Type::Function {
                    params: resolved_params,
                    ret: Box::new(self.resolve(ret, subst, span)),
                }
            }
        }
    }

    /// Map each declared type parameter to a placeholder of the same name.
    pub(super) fn type_param_subst(&self, params: &[Ident]) -> Subst {
        params
            .iter()
            .map(|param| (param.name.clone(), Type::TypeParam(param.name.clone())))
            .collect()
    }

    /// Type-check a call, instantiating generic parameters from explicit type
    /// arguments and/or the actual argument types, and record the instantiation
    /// for IR monomorphization.
    pub(super) fn infer_call(
        &mut self,
        callee_name: Option<&str>,
        callee_ty: &Type,
        args: &[Expr],
        type_args: &[TypeExpr],
        span: Span,
    ) -> Type {
        let resolved = resolve_named_type(callee_ty, &self.aliases);
        let Type::Function { params, ret } = resolved else {
            self.error(
                format!("cannot call a value of type '{}'", callee_ty.display()),
                span,
            );
            return Type::Error;
        };
        let actuals: Vec<Type> = args.iter().map(|arg| self.infer_expr(arg)).collect();
        if params.len() != actuals.len() {
            self.error(
                format!(
                    "expected {} arguments, found {}",
                    params.len(),
                    actuals.len()
                ),
                span,
            );
        }
        let param_names: Vec<String> = callee_name
            .and_then(|name| self.generic_params.get(name).cloned())
            .unwrap_or_default();
        let mut subst = Subst::new();
        if !type_args.is_empty() {
            if type_args.len() != param_names.len() {
                self.error(
                    format!(
                        "expected {} type arguments, found {}",
                        param_names.len(),
                        type_args.len()
                    ),
                    span,
                );
            }
            for (name, expr) in param_names.iter().zip(type_args) {
                subst.insert(name.clone(), self.resolve(expr, &Subst::new(), span));
            }
        }
        for (expected, actual) in params.iter().zip(&actuals) {
            unify(expected, actual, &mut subst, &self.aliases);
        }
        for name in &param_names {
            if !subst.contains_key(name) {
                self.error(format!("cannot infer type parameter '{name}'"), span);
                subst.insert(name.clone(), Type::Unknown);
            }
        }
        for (index, (expected, actual)) in params.iter().zip(&actuals).enumerate() {
            let expected = substitute(expected, &subst);
            if !is_assignable(&expected, actual, &self.aliases) {
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
        self.call_instantiations.insert(
            span,
            param_names.iter().map(|name| subst[name].clone()).collect(),
        );
        substitute(&ret, &subst)
    }
}

/// Bind generic parameters by matching a declared parameter type against the
/// corresponding actual argument type.
fn unify(expected: &Type, actual: &Type, subst: &mut Subst, aliases: &AliasTable) {
    match expected {
        Type::TypeParam(name) => {
            if !subst.contains_key(name) {
                subst.insert(name.clone(), resolve_named_type(actual, aliases));
            }
        }
        Type::Array(inner) => {
            let actual = resolve_named_type(actual, aliases);
            if let Type::Array(actual_inner) = actual {
                unify(inner, &actual_inner, subst, aliases);
            }
        }
        Type::Object(fields) => {
            let actual = resolve_named_type(actual, aliases);
            if let Type::Object(actual_fields) = actual {
                for (name, field_ty) in fields {
                    if let Some(found) = actual_fields.get(name) {
                        unify(field_ty, found, subst, aliases);
                    }
                }
            }
        }
        _ => {}
    }
}
