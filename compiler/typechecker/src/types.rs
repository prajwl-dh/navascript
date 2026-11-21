//! Type resolution and assignability — internal to the type checker.

use std::collections::{BTreeMap, HashMap};

use nava_ast::TypeExpr;
use nava_symbol::Type;

pub type AliasTable = HashMap<String, Type>;

const MAX_ALIAS_DEPTH: u8 = 16;

/// Resolve a syntactic type expression into a semantic `Type`.
pub fn resolve_type(expr: &TypeExpr, aliases: &AliasTable) -> Type {
    resolve(expr, aliases, 0)
}

fn resolve(expr: &TypeExpr, aliases: &AliasTable, depth: u8) -> Type {
    if depth > MAX_ALIAS_DEPTH {
        return Type::Error;
    }
    match expr {
        TypeExpr::Named(name) => match name.as_str() {
            "number" => Type::Number,
            "string" => Type::String,
            "boolean" => Type::Boolean,
            "void" => Type::Void,
            "null" => Type::Null,
            other => aliases
                .get(other)
                .cloned()
                .unwrap_or_else(|| Type::Named(other.to_owned())),
        },
        TypeExpr::Array(inner) => Type::Array(Box::new(resolve(inner, aliases, depth + 1))),
        TypeExpr::Object(fields) => {
            let mut resolved = BTreeMap::new();
            for field in fields {
                resolved.insert(field.name.clone(), resolve(&field.ty, aliases, depth + 1));
            }
            Type::Object(resolved)
        }
        TypeExpr::Function { params, ret } => Type::Function {
            params: params
                .iter()
                .map(|param| resolve(param, aliases, depth + 1))
                .collect(),
            ret: Box::new(resolve(ret, aliases, depth + 1)),
        },
    }
}

/// Whether a value of `actual` may be used where `target` is expected.
///
/// `Unknown` and `Error` are permissive so one bad expression does not cascade
/// into a wall of secondary diagnostics.
pub fn is_assignable(target: &Type, actual: &Type, aliases: &AliasTable) -> bool {
    if target == actual
        || matches!(target, Type::Unknown | Type::Error)
        || matches!(actual, Type::Unknown | Type::Error)
    {
        return true;
    }
    let target = &resolve_named_type(target, aliases);
    let actual = &resolve_named_type(actual, aliases);
    if target == actual
        || matches!(target, Type::Unknown | Type::Error)
        || matches!(actual, Type::Unknown | Type::Error)
    {
        return true;
    }
    match (target, actual) {
        (Type::Array(a), Type::Array(b)) => is_assignable(a, b, aliases),
        (Type::Object(target_fields), Type::Object(actual_fields)) => {
            target_fields.iter().all(|(name, expected)| {
                actual_fields
                    .get(name)
                    .is_some_and(|found| is_assignable(expected, found, aliases))
            })
        }
        (
            Type::Function {
                params: target_params,
                ret: target_ret,
            },
            Type::Function {
                params: actual_params,
                ret: actual_ret,
            },
        ) => {
            target_params.len() == actual_params.len()
                && target_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, found)| is_assignable(expected, found, aliases))
                && is_assignable(target_ret, actual_ret, aliases)
        }
        _ => false,
    }
}

/// Follow alias chains to their target, including inside composite types.
/// A cycle resolves to `Type::Error` rather than looping forever.
pub fn resolve_named_type(ty: &Type, aliases: &AliasTable) -> Type {
    let mut visiting = Vec::new();
    resolve_chain(ty, aliases, &mut visiting)
}

fn resolve_chain(ty: &Type, aliases: &AliasTable, visiting: &mut Vec<String>) -> Type {
    match ty {
        Type::Named(name) => {
            if visiting.iter().any(|seen| seen == name) {
                return Type::Error;
            }
            let Some(target) = aliases.get(name) else {
                return ty.clone();
            };
            visiting.push(name.clone());
            let resolved = resolve_chain(target, aliases, visiting);
            visiting.pop();
            resolved
        }
        Type::Array(inner) => Type::Array(Box::new(resolve_chain(inner, aliases, visiting))),
        Type::Object(fields) => Type::Object(
            fields
                .iter()
                .map(|(name, field_ty)| (name.clone(), resolve_chain(field_ty, aliases, visiting)))
                .collect(),
        ),
        Type::Function { params, ret } => Type::Function {
            params: params
                .iter()
                .map(|param| resolve_chain(param, aliases, visiting))
                .collect(),
            ret: Box::new(resolve_chain(ret, aliases, visiting)),
        },
        _ => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_primitive_and_array_types() {
        let aliases = AliasTable::new();
        assert_eq!(
            resolve_type(
                &TypeExpr::Array(Box::new(TypeExpr::Named("number".into()))),
                &aliases
            ),
            Type::Array(Box::new(Type::Number))
        );
    }

    #[test]
    fn resolves_aliases() {
        let mut aliases = AliasTable::new();
        aliases.insert("Id".to_owned(), Type::Number);
        assert_eq!(
            resolve_type(&TypeExpr::Named("Id".into()), &aliases),
            Type::Number
        );
    }

    #[test]
    fn primitive_mismatch_is_not_assignable() {
        let aliases = AliasTable::new();
        assert!(!is_assignable(&Type::Number, &Type::String, &aliases));
        assert!(is_assignable(&Type::Number, &Type::Number, &aliases));
    }

    #[test]
    fn object_assignability_is_structural() {
        let mut aliases = AliasTable::new();
        let mut expected = BTreeMap::new();
        expected.insert("x".to_owned(), Type::Number);
        let mut actual = BTreeMap::new();
        actual.insert("x".to_owned(), Type::Number);
        actual.insert("y".to_owned(), Type::String);
        assert!(is_assignable(
            &Type::Object(expected.clone()),
            &Type::Object(actual),
            &aliases
        ));
        aliases.insert("Box".to_owned(), Type::Object(expected.clone()));
        assert!(is_assignable(
            &Type::Named("Box".into()),
            &Type::Object(expected),
            &aliases
        ));
    }
}
