//! Generic call monomorphization and name mangling.

use std::collections::HashMap;

use navascript_ast::{ItemKind, Span};
use navascript_symbol::Type;

use super::ir::IrType;
use super::lower::{Lowerer, ir_type_from_symbol};

fn type_key(ty: &IrType) -> String {
    match ty {
        IrType::Number => "number".to_owned(),
        IrType::BigInt => "bigint".to_owned(),
        IrType::Boolean => "boolean".to_owned(),
        IrType::String => "string".to_owned(),
        IrType::Void => "void".to_owned(),
        IrType::Opaque => "opaque".to_owned(),
        IrType::Array(inner) => format!("{}_array", type_key(inner)),
        IrType::Object(fields) => {
            let fields: Vec<String> = fields
                .iter()
                .map(|(name, ty)| {
                    let name: String = name
                        .chars()
                        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
                        .collect();
                    format!("{}_{}", name, type_key(ty))
                })
                .collect();
            format!("object_{}_{}", fields.len(), fields.join("_"))
        }
    }
}

/// Build a stable, structure-sensitive LLVM name for a monomorphization. If a
/// user function already claimed the name, append a numeric suffix.
pub(super) fn mangle(base: &str, args: &[IrType], taken: &dyn Fn(&str) -> bool) -> String {
    let stem = if args.is_empty() {
        base.to_owned()
    } else {
        format!(
            "{}__{}",
            base,
            args.iter().map(type_key).collect::<Vec<_>>().join("_")
        )
    };
    if !taken(&stem) {
        return stem;
    }
    let mut suffix = 1;
    loop {
        let candidate = format!("{stem}_{suffix}");
        if !taken(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

impl Lowerer<'_> {
    /// Return the name to call and its result type, emitting a monomorphization
    /// when the callee is generic. Ordinary calls pass through unchanged.
    pub(super) fn specialize_if_generic(&mut self, name: &str, span: Span) -> (String, IrType) {
        let Some(param_names) = self
            .typed
            .generic_params
            .get(name)
            .filter(|params| !params.is_empty())
            .cloned()
        else {
            let result_ty = if Lowerer::is_known_builtin(name) {
                IrType::Void
            } else {
                self.return_type_of(name)
            };
            return (name.to_owned(), result_ty);
        };
        let concrete: Vec<IrType> = self
            .typed
            .call_instantiations
            .get(&span)
            .map(|types| {
                types
                    .iter()
                    .map(|ty| ir_type_from_symbol(ty, &self.current_subst))
                    .collect()
            })
            .unwrap_or_default();
        if concrete.len() != param_names.len() {
            self.error(
                format!("generic call '{name}' has no inferred type arguments"),
                span,
            );
            return (name.to_owned(), IrType::Opaque);
        }
        if concrete.contains(&IrType::Opaque) {
            self.error(
                format!("cannot resolve a type argument for generic call '{name}'"),
                span,
            );
            return (name.to_owned(), IrType::Opaque);
        }
        let taken = |candidate: &str| self.module.function(candidate).is_some();
        let emitted = mangle(name, &concrete, &taken);
        let subst: HashMap<String, IrType> = param_names
            .iter()
            .cloned()
            .zip(concrete.iter().cloned())
            .collect();
        let key = (name.to_owned(), concrete);
        if !self.specializations.contains_key(&key) {
            let decl = self
                .program
                .items
                .iter()
                .find_map(|item| match item.kind.unwrap_export() {
                    ItemKind::Function(decl) if decl.name.name == name => Some(decl.clone()),
                    _ => None,
                });
            let Some(decl) = decl else {
                self.error(format!("unknown generic function '{name}'"), span);
                return (name.to_owned(), IrType::Opaque);
            };
            // Reserve the name before lowering the body so recursive calls
            // reuse this specialization instead of emitting it forever.
            self.specializations.insert(key, emitted.clone());
            self.lower_function(&decl, Some((emitted.clone(), subst.clone())));
        }
        let result_ty = self
            .typed
            .symbols
            .lookup(name)
            .and_then(|symbol| match &symbol.ty {
                Type::Function { ret, .. } => Some(ir_type_from_symbol(ret, &subst)),
                _ => None,
            })
            .unwrap_or(IrType::Opaque);
        (emitted, result_ty)
    }
}
