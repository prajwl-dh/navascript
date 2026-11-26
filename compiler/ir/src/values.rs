//! IR value representation and type tags, independent of instructions.

/// Identifies an SSA-like instruction result within a function.
pub type ValueId = u32;
/// Identifies a basic block within a function.
pub type BlockId = u32;
/// Identifies a stack slot within a function.
pub type SlotId = u32;
/// Identifies a string constant within a module.
pub type GlobalId = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrType {
    Number,
    BigInt,
    Boolean,
    String,
    Array(Box<IrType>),
    Object(Vec<(String, IrType)>),
    /// A first-class function value: a pointer to a closure object whose first
    /// slot holds the (adapted) code pointer and whose remaining slots hold
    /// captured variables (ADR-0027).
    Function {
        params: Vec<IrType>,
        ret: Box<IrType>,
    },
    Void,
    /// A runtime-tagged value that can hold any primitive or reference.
    Dynamic,
    /// A type whose layout is not fixed until the runtime phase.
    Opaque,
}

impl IrType {
    pub fn display(&self) -> String {
        match self {
            IrType::Number => "number".to_owned(),
            IrType::BigInt => "bigint".to_owned(),
            IrType::Boolean => "boolean".to_owned(),
            IrType::String => "string".to_owned(),
            IrType::Array(inner) => format!("{}[]", inner.display()),
            IrType::Object(_) => "object".to_owned(),
            IrType::Function { params, ret } => format!(
                "({}) => {}",
                params
                    .iter()
                    .map(IrType::display)
                    .collect::<Vec<_>>()
                    .join(", "),
                ret.display()
            ),
            IrType::Void => "void".to_owned(),
            IrType::Dynamic => "any".to_owned(),
            IrType::Opaque => "opaque".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Function parameter, by index.
    Param(u32),
    Local(ValueId),
    Number(f64),
    BigInt(i64),
    Bool(bool),
    String(GlobalId),
    /// Address of a generated NavaScript function, used as the code pointer of
    /// a closure object. The type is carried so call arguments can be typed.
    Function {
        name: String,
        ty: Box<IrType>,
    },
    Null,
}
