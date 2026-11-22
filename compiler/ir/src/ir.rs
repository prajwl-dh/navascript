//! The Nava IR data model — platform-independent by construction.

pub use super::values::{BlockId, GlobalId, IrType, SlotId, Value, ValueId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinaryOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Pow => "**",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }

    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        )
    }

    pub fn is_arithmetic(self) -> bool {
        matches!(
            self,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Pow
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    NegBigInt,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Reserve a typed stack slot. Allocations live in the entry block.
    Alloc {
        slot: SlotId,
        ty: IrType,
    },
    Store {
        slot: SlotId,
        ty: IrType,
        value: Value,
    },
    Load {
        result: ValueId,
        slot: SlotId,
        ty: IrType,
    },
    Binary {
        result: ValueId,
        op: BinaryOp,
        lhs: Value,
        rhs: Value,
    },
    Unary {
        result: ValueId,
        op: UnaryOp,
        operand: Value,
    },
    /// Integer (bigint) binary operation. Arithmetic yields `bigint`,
    /// comparisons and logical ops yield `boolean`.
    BigIntBinary {
        result: ValueId,
        op: BinaryOp,
        lhs: Value,
        rhs: Value,
    },
    /// Pointer null check used by optional chaining.
    IsNull {
        result: ValueId,
        value: Value,
    },
    /// JavaScript-style truthiness conversion for non-boolean conditions.
    ToBool {
        result: ValueId,
        value: Value,
    },
    /// Direct call. Indirect calls are not supported in Phase 3.
    Call {
        result: Option<ValueId>,
        result_ty: IrType,
        callee: String,
        args: Vec<Value>,
    },
    /// Concatenate two runtime strings.
    StringConcat {
        result: ValueId,
        left: Value,
        right: Value,
    },
    /// Convert a number to its string representation for concatenation.
    NumberToString {
        result: ValueId,
        value: Value,
    },
    StringEquals {
        result: ValueId,
        left: Value,
        right: Value,
    },
    StringLength {
        result: ValueId,
        value: Value,
    },
    StringCharAt {
        result: ValueId,
        value: Value,
        index: Value,
    },
    /// Allocate an array whose slots are all of `element_ty`.
    ArrayNew {
        result: ValueId,
        element_ty: IrType,
        length: u32,
    },
    ArrayGet {
        result: ValueId,
        array: Value,
        index: Value,
        element_ty: IrType,
    },
    ArraySet {
        array: Value,
        index: Value,
        value: Value,
        element_ty: IrType,
    },
    ArrayLength {
        result: ValueId,
        array: Value,
    },
    /// Allocate an object with the given field names and types.
    ObjectNew {
        result: ValueId,
        fields: Vec<(String, IrType)>,
    },
    ObjectGet {
        result: ValueId,
        object: Value,
        field_index: u32,
        field_ty: IrType,
    },
    ObjectSet {
        object: Value,
        field_index: u32,
        value: Value,
        field_ty: IrType,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    Return(Option<Value>),
    Br(BlockId),
    CondBr {
        cond: Value,
        then_block: BlockId,
        else_block: BlockId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub label: String,
    pub instructions: Vec<Instruction>,
    pub terminator: Option<Terminator>,
}

impl BasicBlock {
    pub fn new(id: BlockId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            instructions: Vec::new(),
            terminator: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrParam {
    pub name: String,
    pub ty: IrType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<IrParam>,
    pub return_ty: IrType,
    pub entry: BlockId,
    pub blocks: Vec<BasicBlock>,
}

impl IrFunction {
    pub fn block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|block| block.id == id)
    }

    pub fn block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|block| block.id == id)
    }

    pub fn remove_block(&mut self, id: BlockId) {
        self.blocks.retain(|block| block.id != id);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrGlobal {
    pub id: GlobalId,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub globals: Vec<IrGlobal>,
}

impl IrModule {
    pub fn function(&self, name: &str) -> Option<&IrFunction> {
        self.functions.iter().find(|function| function.name == name)
    }
}
