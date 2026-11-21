//! Tests for the symbol table leaf.

use std::collections::BTreeMap;

use nava_ast::{FileId, Span};
use nava_symbol::{Symbol, SymbolKind, SymbolTable, Type};

fn symbol(name: &str, ty: Type) -> Symbol {
    Symbol {
        name: name.to_owned(),
        kind: SymbolKind::Variable,
        ty,
        span: Span::new(FileId(0), 0, 1),
    }
}

#[test]
fn lookup_finds_current_scope() {
    let mut table = SymbolTable::new();
    table.insert(symbol("x", Type::Number));
    assert_eq!(table.lookup("x").unwrap().ty, Type::Number);
    assert!(table.lookup("missing").is_none());
}

#[test]
fn inner_scope_shadows_outer_and_is_dropped() {
    let mut table = SymbolTable::new();
    table.insert(symbol("x", Type::Number));
    table.enter_scope();
    table.insert(symbol("x", Type::String));
    assert_eq!(table.lookup("x").unwrap().ty, Type::String);
    table.leave_scope();
    assert_eq!(table.lookup("x").unwrap().ty, Type::Number);
}

#[test]
fn insert_reports_a_duplicate_in_the_same_scope() {
    let mut table = SymbolTable::new();
    assert!(table.insert(symbol("x", Type::Number)).is_none());
    assert!(table.insert(symbol("x", Type::String)).is_some());
}

#[test]
fn displays_composite_types() {
    assert_eq!(Type::Array(Box::new(Type::Number)).display(), "number[]");
    let mut fields = BTreeMap::new();
    fields.insert("x".to_owned(), Type::Number);
    assert_eq!(Type::Object(fields).display(), "{ x: number }");
}
