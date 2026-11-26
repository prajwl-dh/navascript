//! Optimizer unit tests: branch folding and unreachable-block removal.

use navascript_ir::{
    BasicBlock, BinaryOp, Instruction, IrFunction, IrModule, IrType, Terminator, Value, verify,
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
fn branch_folding_removes_unreachable_blocks() {
    let entry = BasicBlock {
        id: 0,
        label: "entry".to_owned(),
        instructions: vec![Instruction::Binary {
            result: 0,
            op: BinaryOp::Eq,
            lhs: Value::Number(1.0),
            rhs: Value::Number(1.0),
        }],
        terminator: Some(Terminator::CondBr {
            cond: Value::Local(0),
            then_block: 1,
            else_block: 2,
        }),
    };
    let then_block = BasicBlock {
        id: 1,
        label: "then".to_owned(),
        instructions: Vec::new(),
        terminator: Some(Terminator::Return(Some(Value::Number(1.0)))),
    };
    let else_block = BasicBlock {
        id: 2,
        label: "else".to_owned(),
        instructions: Vec::new(),
        terminator: Some(Terminator::Return(Some(Value::Number(2.0)))),
    };
    let module = IrModule {
        functions: vec![IrFunction {
            name: "navascript_main".to_owned(),
            params: Vec::new(),
            return_ty: IrType::Number,
            entry: 0,
            blocks: vec![entry, then_block, else_block],
        }],
        globals: Vec::new(),
    };

    let result = optimize(&module, Level::Minimal);
    assert!(
        verify(&result.module).is_empty(),
        "{:?}",
        verify(&result.module)
    );
    assert_eq!(
        result.module.functions[0].blocks.len(),
        2,
        "dead branch should go"
    );
    assert!(result.stats.eliminated >= 1);
}

#[test]
fn escape_analysis_promotes_a_non_escaping_slot() {
    let module = IrModule {
        functions: vec![IrFunction {
            name: "navascript_main".to_owned(),
            params: Vec::new(),
            return_ty: IrType::Number,
            entry: 0,
            blocks: vec![block(
                vec![
                    Instruction::Alloc {
                        slot: 0,
                        ty: IrType::Number,
                    },
                    Instruction::Store {
                        slot: 0,
                        ty: IrType::Number,
                        value: Value::Number(5.0),
                    },
                    Instruction::Load {
                        result: 1,
                        slot: 0,
                        ty: IrType::Number,
                    },
                ],
                Terminator::Return(Some(Value::Local(1))),
            )],
        }],
        globals: Vec::new(),
    };

    let result = optimize(&module, Level::Aggressive);
    assert!(
        verify(&result.module).is_empty(),
        "{:?}",
        verify(&result.module)
    );
    assert_eq!(result.stats.promoted, 1);
    assert!(result.module.functions[0].blocks[0].instructions.is_empty());
    assert_eq!(
        result.module.functions[0].blocks[0].terminator,
        Some(Terminator::Return(Some(Value::Number(5.0))))
    );
}
