//! Tests for type resolution and assignability through public seams.

use std::collections::BTreeMap;

use nava_ast::TypeExpr;
use nava_symbol::Type;
use nava_typechecker::{AliasTable, is_assignable, resolve_type};

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
