//! Expressions and operators.

use super::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Number(f64),
    /// A `123n` literal. Nava currently represents bigints as signed 64-bit
    /// integers; wider values are diagnosed rather than silently wrapped.
    BigInt(i64),
    String(String),
    Bool(bool),
    Null,
    Ident(String),
    /// `this` inside a class method.
    This,
    /// `super` inside a class method (the base-class instance).
    Super,
    /// `new Class(args)`.
    New {
        class: super::types::Ident,
        args: Vec<Expr>,
    },
    /// `super(args)` — calls the base-class constructor on `this`.
    SuperCall {
        args: Vec<Expr>,
    },
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
    CompoundAssign {
        op: AssignOp,
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Update {
        op: UpdateOp,
        operand: Box<Expr>,
        prefix: bool,
    },
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },
    Arrow {
        params: Vec<super::item::Param>,
        return_ty: Option<super::types::TypeAnnotation>,
        body: ArrowBody,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        /// Explicit type arguments (`f<number>(1)`); empty when inferred.
        type_args: Vec<super::types::TypeExpr>,
        /// `a?.()` vs `a()`. The null branch is unobservable until nullable
        /// callable values exist, so lowering treats both as a direct call.
        optional: bool,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        optional: bool,
    },
    ArrayLit(Vec<Expr>),
    ObjectLit(Vec<ObjectField>),
    Member {
        object: Box<Expr>,
        property: String,
        optional: bool,
    },
    Paren(Box<Expr>),
}

/// The body of an arrow function: a single expression or a block.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrowBody {
    Expr(Box<Expr>),
    Block(super::item::Block),
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
    Pow,
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

/// The arithmetic operators available in compound assignments (`+=` etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl AssignOp {
    pub fn binary_op(self) -> BinaryOp {
        match self {
            AssignOp::Add => BinaryOp::Add,
            AssignOp::Sub => BinaryOp::Sub,
            AssignOp::Mul => BinaryOp::Mul,
            AssignOp::Div => BinaryOp::Div,
            AssignOp::Mod => BinaryOp::Mod,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOp {
    Inc,
    Dec,
}

impl UpdateOp {
    pub fn symbol(self) -> &'static str {
        match self {
            UpdateOp::Inc => "++",
            UpdateOp::Dec => "--",
        }
    }
}
