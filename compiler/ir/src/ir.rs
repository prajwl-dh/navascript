//! The Nava IR data model — platform-independent by construction.

/// Identifies an SSA-like instruction result within a function.
pub type ValueId = u32;
/// Identifies a basic block within a function.
pub type BlockId = u32;
/// Identifies a stack slot within a function.
pub type SlotId = u32;
/// Identifies a string constant within a module.
pub type GlobalId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrType {
    Number,
    Boolean,
    String,
    Void,
    /// A type whose layout is not fixed until the runtime phase.
    Opaque,
}

impl IrType {
    pub fn display(&self) -> &'static str {
        match self {
            IrType::Number => "number",
            IrType::Boolean => "boolean",
            IrType::String => "string",
            IrType::Void => "void",
            IrType::Opaque => "opaque",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Function parameter, by index.
    Param(u32),
    Local(ValueId),
    Number(f64),
    Bool(bool),
    String(GlobalId),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
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
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Reserve a typed stack slot. Allocations live in the entry block.
    Alloc { slot: SlotId, ty: IrType },
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
    /// JavaScript-style truthiness conversion for non-boolean conditions.
    ToBool { result: ValueId, value: Value },
    /// Direct call. Indirect calls are not supported in Phase 3.
    Call {
        result: Option<ValueId>,
        result_ty: IrType,
        callee: String,
        args: Vec<Value>,
    },
    Index {
        result: ValueId,
        object: Value,
        index: Value,
    },
    Field {
        result: ValueId,
        object: Value,
        name: String,
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
