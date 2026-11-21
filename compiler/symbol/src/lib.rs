//! Nava types and lexical scopes — the symbol-table internal seam.
//!
//! This crate is a dependency-free leaf (apart from `nava-ast`) so the type
//! checker can own one `SymbolTable` behind its own external interface.

use std::collections::{BTreeMap, HashMap};

use nava_ast::Span;

/// The type language of the Phase 2 checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Number,
    String,
    Boolean,
    Void,
    Null,
    Array(Box<Type>),
    Object(BTreeMap<String, Type>),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    /// A named type awaiting alias/interface resolution.
    Named(String),
    /// No information yet (inference fallback).
    Unknown,
    /// A type that already produced a diagnostic.
    Error,
}

impl Type {
    pub fn display(&self) -> String {
        match self {
            Type::Number => "number".to_owned(),
            Type::String => "string".to_owned(),
            Type::Boolean => "boolean".to_owned(),
            Type::Void => "void".to_owned(),
            Type::Null => "null".to_owned(),
            Type::Array(inner) => format!("{}[]", inner.display()),
            Type::Object(fields) => {
                let body = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", ty.display()))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("{{ {body} }}")
            }
            Type::Function { params, ret } => {
                let params = params
                    .iter()
                    .map(Type::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({params}) => {}", ret.display())
            }
            Type::Named(name) => name.clone(),
            Type::Unknown => "unknown".to_owned(),
            Type::Error => "<error>".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Parameter,
    Function,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub span: Span,
}

/// Lexical scopes, innermost last. Insertion into the current scope shadows
/// outer bindings; lookup walks from innermost to outermost.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn leave_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Insert into the current scope, returning any symbol it replaced.
    pub fn insert(&mut self, symbol: Symbol) -> Option<Symbol> {
        self.scopes
            .last_mut()
            .expect("always at least one scope")
            .insert(symbol.name.clone(), symbol)
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nava_ast::FileId;

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
}
