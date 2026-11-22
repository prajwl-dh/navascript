//! Type syntax and named declarations.

use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAnnotation {
    pub expr: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    Named(String),
    /// A generic type reference such as `Box<number>`.
    Generic {
        name: String,
        args: Vec<TypeExpr>,
    },
    Array(Box<TypeExpr>),
    /// `A | B` — a value may be any one of the members.
    Union(Vec<TypeExpr>),
    /// `A & B` — a value must satisfy every member.
    Intersection(Vec<TypeExpr>),
    Object(Vec<TypeField>),
    Function {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeField {
    pub name: String,
    pub ty: TypeExpr,
    /// `field?: T` — the field may be absent from an object literal.
    pub optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDecl {
    pub name: Ident,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub type_params: Vec<Ident>,
    pub fields: Vec<TypeField>,
    pub span: Span,
}
