//! Nava abstract syntax tree — a dependency-free leaf crate.
//!
//! `Program`, `Span` and `FileId` are shared with the type checker and IR
//! lowering in later phases, so this crate depends on nothing else.

/// Identifies a source file. Phase 1 parses a single file, so `FileId(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// A half-open byte range in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file: FileId, start: usize, end: usize) -> Self {
        Self {
            file,
            start: start as u32,
            end: end as u32,
        }
    }

    pub fn len(&self) -> u32 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A parsed source file.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ItemKind {
    Variable(VariableDecl),
    Function(FunctionDecl),
    TypeAlias(TypeAliasDecl),
    Interface(InterfaceDecl),
    Expr(Expr),
}

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
    Array(Box<TypeExpr>),
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
    pub fields: Vec<TypeField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub name: Ident,
    pub ty: Option<TypeAnnotation>,
    pub init: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeAnnotation>,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Expr(Expr),
    Variable(VariableDecl),
    Return(Option<Expr>),
    If(IfStmt),
    Block(Block),
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Block,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    If(Box<IfStmt>),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Ident(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    ArrayLit(Vec<Expr>),
    ObjectLit(Vec<ObjectField>),
    Member {
        object: Box<Expr>,
        property: String,
    },
    Paren(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    StrictEq,
    NotEq,
    StrictNotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Neg,
    Pos,
}
