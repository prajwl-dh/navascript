//! Optimizer unit tests: folding and dead-code elimination.

use navascript_ir::{
    BasicBlock, BinaryOp, Instruction, IrFunction, IrModule, IrParam, IrType, Terminator, Value,
    verify,
};
use navascript_optimizer::{Level, optimize};

fn block(instructions: Vec<Instruction>, terminator: Terminator) -> BasicBlock {
    BasicBlock {
        id: 0,
        label: "bb0".to_owned(),
        instructions,
        terminator: Some(terminator),
    }
}

#[test]
fn no_opt_level_is_an_identity_pass() {
    let module = IrModule::default();
    let result = optimize(&module, Level::None);
    assert_eq!(result.module, module);
    assert_eq!(
        result.stats.folded + result.stats.eliminated + result.stats.inlined,
        0
    );
}

#[test]
fn folding_removes_constant_expressions_and_rewrites_uses() {
    let module = IrModule {
        functions: vec![IrFunction {
            name: "navascript_main".to_owned(),
            params: Vec::new(),
            return_ty: IrType::Number,
            entry: 0,
            blocks: vec![block(
                vec![
                    Instruction::Binary {
                        result: 0,
                        op: BinaryOp::Add,
                        lhs: Value::Number(2.0),
                        rhs: Value::Number(3.0),
                    },
                    Instruction::Binary {
                        result: 1,
                        op: BinaryOp::Mul,
                        lhs: Value::Local(0),
                        rhs: Value::Number(4.0),
                    },
                    Instruction::Binary {
                        result: 2,
                        op: BinaryOp::Sub,
                        lhs: Value::Local(1),
                        rhs: Value::Number(1.0),
                    },
                ],
                Terminator::Return(Some(Value::Local(1))),
            )],
        }],
        globals: Vec::new(),
    };

    let result = optimize(&module, Level::Minimal);
    assert!(
        verify(&result.module).is_empty(),
        "{:?}",
        verify(&result.module)
    );
    assert!(result.module.functions[0].blocks[0].instructions.is_empty());
    assert_eq!(
        result.module.functions[0].blocks[0].terminator,
        Some(Terminator::Return(Some(Value::Number(20.0))))
    );
    assert_eq!(result.stats.folded, 3);
}

#[test]
fn dead_code_elimination_keeps_used_pure_instructions() {
    let module = IrModule {
        functions: vec![IrFunction {
            name: "navascript_main".to_owned(),
            return_ty: IrType::Number,
            entry: 0,
            blocks: vec![block(
                vec![
                    Instruction::Binary {
                        result: 1,
                        op: BinaryOp::Add,
                        lhs: Value::Param(0),
                        rhs: Value::Number(1.0),
                    },
                    Instruction::Binary {
                        result: 2,
                        op: BinaryOp::Mul,
                        lhs: Value::Param(0),
                        rhs: Value::Number(3.0),
                    },
                ],
                Terminator::Return(Some(Value::Local(1))),
            )],
            // A parameter makes the first instruction unfoldsable.
            params: vec![IrParam {
                name: "n".to_owned(),
                ty: IrType::Number,
            }],
        }],
        globals: Vec::new(),
    };

    let result = optimize(&module, Level::Minimal);
    assert!(
        verify(&result.module).is_empty(),
        "{:?}",
        verify(&result.module)
    );
    let instructions = &result.module.functions[0].blocks[0].instructions;
    assert_eq!(instructions.len(), 1, "only the dead constant should go");
    assert_eq!(instructions[0].result(), Some(1));
    assert_eq!(result.stats.eliminated, 1);
}
