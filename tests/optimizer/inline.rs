//! Optimizer unit tests: inlining, result renaming and escape analysis.

use navascript_ir::{
    BasicBlock, Instruction, IrFunction, IrModule, IrParam, IrType, Terminator, Value, verify,
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
fn inlining_substitutes_arguments_and_preserves_the_call_result() {
    let sink = IrFunction {
        name: "navascript_sink".to_owned(),
        params: vec![IrParam {
            name: "n".to_owned(),
            ty: IrType::Number,
        }],
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
                    value: Value::Param(0),
                },
                Instruction::Load {
                    result: 1,
                    slot: 0,
                    ty: IrType::Number,
                },
            ],
            Terminator::Return(Some(Value::Local(1))),
        )],
    };
    let caller = IrFunction {
        name: "navascript_main".to_owned(),
        params: Vec::new(),
        return_ty: IrType::Number,
        entry: 0,
        blocks: vec![block(
            vec![Instruction::Call {
                result: Some(1),
                result_ty: IrType::Number,
                callee: sink.name.clone(),
                args: vec![Value::Number(7.0)],
            }],
            Terminator::Return(Some(Value::Local(1))),
        )],
    };
    let module = IrModule {
        functions: vec![caller, sink],
        globals: Vec::new(),
    };

    let result = optimize(&module, Level::Aggressive);
    assert!(
        verify(&result.module).is_empty(),
        "{:?}",
        verify(&result.module)
    );
    assert_eq!(result.stats.inlined, 1);
    let instructions = &result.module.functions[0].blocks[0].instructions;
    assert!(
        !instructions
            .iter()
            .any(|item| matches!(item, Instruction::Call { .. })),
        "call should be replaced by the callee body"
    );
    // Escape analysis then promotes the inlined slot, so the argument must
    // survive as the returned constant.
    assert_eq!(
        result.module.functions[0].blocks[0].terminator,
        Some(Terminator::Return(Some(Value::Number(7.0)))),
        "argument should be substituted and returned"
    );
}

#[test]
fn inlining_renames_result_ids_that_differ_from_the_call_result() {
    let sink = IrFunction {
        name: "navascript_sink".to_owned(),
        params: vec![IrParam {
            name: "n".to_owned(),
            ty: IrType::Number,
        }],
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
                    value: Value::Param(0),
                },
                Instruction::Load {
                    result: 3,
                    slot: 0,
                    ty: IrType::Number,
                },
            ],
            Terminator::Return(Some(Value::Local(3))),
        )],
    };
    let caller = IrFunction {
        name: "navascript_main".to_owned(),
        params: Vec::new(),
        return_ty: IrType::Number,
        entry: 0,
        blocks: vec![block(
            vec![Instruction::Call {
                result: Some(5),
                result_ty: IrType::Number,
                callee: sink.name.clone(),
                args: vec![Value::Number(9.0)],
            }],
            Terminator::Return(Some(Value::Local(5))),
        )],
    };
    let module = IrModule {
        functions: vec![caller, sink],
        globals: Vec::new(),
    };

    let result = optimize(&module, Level::Aggressive);
    assert_eq!(
        result.stats.inlined, 1,
        "inlining must not be silently discarded"
    );
    assert!(
        verify(&result.module).is_empty(),
        "{:?}",
        verify(&result.module)
    );
    assert_eq!(
        result.module.functions[0].blocks[0].terminator,
        Some(Terminator::Return(Some(Value::Number(9.0)))),
        "the call result must hold the inlined callee's return value"
    );
}
