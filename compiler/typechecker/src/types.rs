//! Type resolution and assignability — internal to the type checker.

use std::collections::{BTreeMap, HashMap};

use navascript_ast::TypeExpr;
use navascript_symbol::Type;

pub type AliasTable = HashMap<String, Type>;
/// Maps a generic type parameter name to its instantiated type.
pub type Subst = HashMap<String, Type>;

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
            "bigint" => Type::BigInt,
            "string" => Type::String,
            "boolean" => Type::Boolean,
            "void" => Type::Void,
            "null" => Type::Null,
            other => aliases
                .get(other)
                .cloned()
                .unwrap_or_else(|| Type::Named(other.to_owned())),
        },
        // Generic type references need the checker's declaration registry to
        // know a generic's parameter names and to emit diagnostics; only
        // `Checker::resolve` handles them. This free function is kept for the
        // non-generic public API and never sees generic syntax internally.
        TypeExpr::Generic { .. } => Type::Error,
        TypeExpr::Union(members) => Type::Union(
            members
                .iter()
                .map(|member| resolve(member, aliases, depth + 1))
                .collect(),
        ),
        TypeExpr::Intersection(members) => Type::Intersection(
            members
                .iter()
                .map(|member| resolve(member, aliases, depth + 1))
                .collect(),
        ),
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
        (Type::Union(targets), Type::Union(actuals)) => actuals.iter().all(|actual_member| {
            targets
                .iter()
                .any(|target_member| is_assignable(target_member, actual_member, aliases))
        }),
        (Type::Union(targets), _) => targets
            .iter()
            .any(|member| is_assignable(member, actual, aliases)),
        (_, Type::Union(actuals)) => actuals
            .iter()
            .all(|member| is_assignable(target, member, aliases)),
        (Type::Intersection(targets), _) => targets
            .iter()
            .all(|member| is_assignable(member, actual, aliases)),
        (_, Type::Intersection(actuals)) => actuals
            .iter()
            .any(|member| is_assignable(target, member, aliases)),
        (Type::ClassInstance { fields, .. }, other)
        | (other, Type::ClassInstance { fields, .. }) => {
            let other = resolve_named_type(other, aliases);
            let other_fields = match other {
                Type::Object(map) => map.into_iter().collect::<Vec<(String, Type)>>(),
                Type::ClassInstance { fields, .. } => fields,
                Type::Unknown | Type::Error => return true,
                _ => return false,
            };
            fields.iter().all(|(name, ty)| {
                other_fields
                    .iter()
                    .find(|(other_name, _)| other_name == name)
                    .is_some_and(|(_, other_ty)| is_assignable(ty, other_ty, aliases))
            })
        }
        (Type::Array(a), Type::Array(b)) => is_assignable(a, b, aliases),
        (Type::Object(target_fields), Type::Object(actual_fields)) => {
            target_fields.iter().all(|(name, expected)| {
                match actual_fields.get(name) {
                    Some(found) => is_assignable(expected, found, aliases),
                    // An optional property resolves to `T | null`; a missing
                    // field is assignable to it.
                    None => matches!(expected, Type::Union(members)
                        if members.iter().any(|member| matches!(member, Type::Null))),
                }
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
    normalize(&resolve_chain(ty, aliases, &mut visiting))
}

/// Flatten nested unions/intersections, drop duplicates, and collapse a single
/// member. `Unknown`/`Error` absorb a union so one bad type does not cascade.
fn normalize(ty: &Type) -> Type {
    fn flatten(members: &[Type], intersection: bool) -> Vec<Type> {
        let mut out = Vec::new();
        for member in members {
            match normalize(member) {
                Type::Union(inner) if !intersection => out.extend(inner),
                Type::Intersection(inner) if intersection => out.extend(inner),
                other => out.push(other),
            }
        }
        out.dedup();
        out
    }
    match ty {
        Type::Union(members) => {
            let members = flatten(members, false);
            if members.len() == 1 {
                members.into_iter().next().unwrap()
            } else {
                Type::Union(members)
            }
        }
        Type::Intersection(members) => {
            let members = flatten(members, true);
            if members.len() == 1 {
                members.into_iter().next().unwrap()
            } else {
                Type::Intersection(members)
            }
        }
        Type::Array(inner) => Type::Array(Box::new(normalize(inner))),
        _ => ty.clone(),
    }
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
        Type::Union(members) => Type::Union(
            members
                .iter()
                .map(|member| resolve_chain(member, aliases, visiting))
                .collect(),
        ),
        Type::Intersection(members) => Type::Intersection(
            members
                .iter()
                .map(|member| resolve_chain(member, aliases, visiting))
                .collect(),
        ),
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

/// Look up a field on object-like types, including intersections and class
/// instances, so every consumer shares one resolution rule.
pub fn object_field(ty: &Type, property: &str, aliases: &AliasTable) -> Option<Type> {
    match resolve_named_type(ty, aliases) {
        Type::Object(fields) => fields.get(property).cloned(),
        Type::ClassInstance { fields, .. } => fields
            .iter()
            .find(|(name, _)| name == property)
            .map(|(_, ty)| ty.clone()),
        Type::Intersection(members) => members
            .iter()
            .find_map(|member| object_field(member, property, aliases)),
        _ => None,
    }
}

/// Reject unions whose members cannot share one native representation. The
/// backend has tagged-free unboxed values, so a `number | string` or a nullable
/// `number` cannot be stored safely (README: never silently miscompile).
pub fn union_layout_error(ty: &Type, aliases: &AliasTable) -> Option<String> {
    let Type::Union(members) = resolve_named_type(ty, aliases) else {
        return None;
    };
    let has_null = members.iter().any(|member| matches!(member, Type::Null));
    let mut categories = std::collections::BTreeSet::new();
    for member in &members {
        let category = match member {
            Type::Null => continue,
            Type::Number => "number",
            Type::BigInt => "bigint",
            Type::Boolean => "boolean",
            Type::String | Type::Array(_) | Type::Object(_) | Type::ClassInstance { .. } => {
                "reference"
            }
            _ => "other",
        };
        categories.insert(category);
    }
    if categories.len() > 1 {
        return Some("union members must share a runtime representation".to_owned());
    }
    if has_null && !categories.is_empty() && !categories.contains("reference") {
        return Some("nullable unions require a reference type".to_owned());
    }
    None
}

/// Replace generic type parameters with their instantiated types, recursively.
pub fn substitute(ty: &Type, subst: &Subst) -> Type {
    match ty {
        Type::TypeParam(name) => subst.get(name).cloned().unwrap_or_else(|| ty.clone()),
        Type::Array(inner) => Type::Array(Box::new(substitute(inner, subst))),
        Type::Object(fields) => Type::Object(
            fields
                .iter()
                .map(|(name, field_ty)| (name.clone(), substitute(field_ty, subst)))
                .collect(),
        ),
        Type::Function { params, ret } => Type::Function {
            params: params
                .iter()
                .map(|param| substitute(param, subst))
                .collect(),
            ret: Box::new(substitute(ret, subst)),
        },
        _ => ty.clone(),
    }
}
