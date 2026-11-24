//! NavaScript types and lexical scopes — the symbol-table internal seam.
//!
//! This crate is a dependency-free leaf (apart from `navascript-ast`) so the type
//! checker can own one `SymbolTable` behind its own external interface.

use std::collections::{BTreeMap, HashMap};

use navascript_ast::Span;

/// The type language of the Phase 2 checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Number,
    /// The `bigint` primitive. NavaScript currently stores it as a signed 64-bit
    /// integer; wider literals are diagnosed during parsing.
    BigInt,
    String,
    Boolean,
    Void,
    Null,
    Array(Box<Type>),
    Union(Vec<Type>),
    Intersection(Vec<Type>),
    Object(BTreeMap<String, Type>),
    /// A class instance. `fields` is in base-first declaration order so every
    /// method in the hierarchy agrees on slot indices (ADR-0024).
    ClassInstance {
        name: String,
        fields: Vec<(String, Type)>,
    },
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    /// A promise value wrapping a result type (ADR-0029). NavaScript promises
    /// are synchronous: `await` unwraps them without suspending.
    Promise(Box<Type>),
    /// A named type awaiting alias/interface resolution.
    Named(String),
    /// A generic type parameter, bound at a call site (ADR-0023).
    TypeParam(String),
    /// No information yet (inference fallback).
    Unknown,
    /// A type that already produced a diagnostic.
    Error,
}

impl Type {
    pub fn display(&self) -> String {
        match self {
            Type::Number => "number".to_owned(),
            Type::BigInt => "bigint".to_owned(),
            Type::String => "string".to_owned(),
            Type::Boolean => "boolean".to_owned(),
            Type::Void => "void".to_owned(),
            Type::Null => "null".to_owned(),
            Type::Array(inner) => format!("{}[]", inner.display()),
            Type::Union(members) => members
                .iter()
                .map(Type::display)
                .collect::<Vec<_>>()
                .join(" | "),
            Type::Intersection(members) => members
                .iter()
                .map(Type::display)
                .collect::<Vec<_>>()
                .join(" & "),
            Type::Object(fields) => {
                let body = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", ty.display()))
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("{{ {body} }}")
            }
            Type::ClassInstance { name, .. } => name.clone(),
            Type::Function { params, ret } => {
                let params = params
                    .iter()
                    .map(Type::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({params}) => {}", ret.display())
            }
            Type::Promise(inner) => format!("Promise<{}>", inner.display()),
            Type::Named(name) => name.clone(),
            Type::TypeParam(name) => name.clone(),
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
