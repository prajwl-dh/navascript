//! Tests for the IR verifier's structural, dominance and type checks.

use navascript_ir::{
    BasicBlock, BinaryOp, Instruction, IrFunction, IrModule, IrParam, IrType, Terminator, Value,
    verify,
};

fn function(name: &str) -> IrFunction {
    IrFunction {
        name: name.to_owned(),
        params: vec![IrParam {
            name: "n".to_owned(),
            ty: IrType::Number,
        }],
        return_ty: IrType::Number,
        entry: 0,
        blocks: vec![BasicBlock::new(0, "entry")],
    }
}

fn finish(mut function: IrFunction) -> IrModule {
    if function.blocks[0].terminator.is_none() {
        function.blocks[0].terminator = Some(Terminator::Return(Some(Value::Number(1.0))));
    }
    IrModule {
        functions: vec![function],
        globals: Vec::new(),
    }
}

#[test]
fn accepts_a_well_formed_function() {
    let module = finish(function("f"));
    assert!(verify(&module).is_empty());
}

#[test]
fn rejects_bigint_binary_with_number_operands() {
    let mut function = function("f");
    function.blocks[0]
        .instructions
        .push(Instruction::BigIntBinary {
            result: 1,
            op: BinaryOp::Add,
            lhs: Value::Param(0),
            rhs: Value::BigInt(1),
        });
    let errors = verify(&finish(function));
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("expected 'bigint', found 'number'")),
        "{errors:?}"
    );
}

#[test]
fn rejects_bigint_negation_of_a_number() {
    let mut function = function("f");
    function.blocks[0].instructions.push(Instruction::Unary {
        result: 1,
        op: navascript_ir::UnaryOp::NegBigInt,
        operand: Value::Param(0),
    });
    let errors = verify(&finish(function));
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("expected 'bigint', found 'number'")),
        "{errors:?}"
    );
}

#[test]
fn rejects_a_block_without_a_terminator() {
    let mut function = function("f");
    function.blocks[0].terminator = None;
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("no terminator"))
    );
}

#[test]
fn rejects_a_branch_to_an_unknown_block() {
    let mut function = function("f");
    function.blocks[0].terminator = Some(Terminator::Br(42));
    let errors = verify(&finish(function));
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("unknown block"))
    );
}

#[test]
fn rejects_use_of_an_undefined_value() {
    let mut function = function("f");
    function.blocks[0].instructions.push(Instruction::Binary {
        result: 3,
        op: BinaryOp::Add,
        lhs: Value::Local(99),
        rhs: Value::Number(1.0),
    });
    function.blocks[0].terminator = Some(Terminator::Return(Some(Value::Local(3))));
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("not defined here"))
    );
}

#[test]
fn rejects_type_mismatched_arithmetic() {
    let mut function = function("f");
    function.blocks[0].instructions.push(Instruction::Binary {
        result: 3,
        op: BinaryOp::Add,
        lhs: Value::Bool(true),
        rhs: Value::Number(1.0),
    });
    function.blocks[0].terminator = Some(Terminator::Return(Some(Value::Local(3))));
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("expected 'number'"))
    );
}

#[test]
fn rejects_a_value_used_where_its_definition_does_not_dominate() {
    let mut function = function("f");
    function.blocks[0].terminator = Some(Terminator::CondBr {
        cond: Value::Bool(true),
        then_block: 1,
        else_block: 2,
    });

    // Define %7 only on the `then` branch...
    let mut then_block = BasicBlock::new(1, "then");
    then_block.instructions.push(Instruction::Binary {
        result: 7,
        op: BinaryOp::Add,
        lhs: Value::Number(1.0),
        rhs: Value::Number(2.0),
    });
    then_block.terminator = Some(Terminator::Br(3));

    // ...and use it on the `else` branch, which `then` does not dominate.
    let mut else_block = BasicBlock::new(2, "else");
    else_block.instructions.push(Instruction::Binary {
        result: 3,
        op: BinaryOp::Add,
        lhs: Value::Local(7),
        rhs: Value::Number(1.0),
    });
    else_block.terminator = Some(Terminator::Br(3));

    let mut join = BasicBlock::new(3, "join");
    join.terminator = Some(Terminator::Return(Some(Value::Number(0.0))));

    function.blocks.extend([then_block, else_block, join]);
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("not defined here"))
    );
}

#[test]
fn rejects_use_before_definition_in_the_same_block() {
    let mut function = function("f");
    function.blocks[0].instructions.push(Instruction::Binary {
        result: 3,
        op: BinaryOp::Add,
        lhs: Value::Local(4),
        rhs: Value::Number(1.0),
    });
    function.blocks[0].instructions.push(Instruction::Binary {
        result: 4,
        op: BinaryOp::Add,
        lhs: Value::Number(1.0),
        rhs: Value::Number(2.0),
    });
    function.blocks[0].terminator = Some(Terminator::Return(Some(Value::Local(3))));
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("not defined here"))
    );
}

#[test]
fn rejects_an_out_of_range_parameter() {
    let mut function = function("f");
    function.blocks[0].terminator = Some(Terminator::Return(Some(Value::Param(9))));
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("parameter"))
    );
}

#[test]
fn rejects_an_unknown_string_global() {
    let mut function = function("f");
    function.blocks[0].terminator = Some(Terminator::Return(Some(Value::String(42))));
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("unknown string global"))
    );
}

#[test]
fn rejects_wrong_return_type() {
    let mut function = function("f");
    function.blocks[0].terminator = Some(Terminator::Return(Some(Value::Bool(true))));
    let errors = verify(&IrModule {
        functions: vec![function],
        globals: Vec::new(),
    });
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("returns 'boolean'"))
    );
}
