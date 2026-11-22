//! Golden-style tests for AST → IR lowering via the public `lower` seam.

use nava_ir::{Instruction, Terminator, Value, lower};
use nava_parser::parse;
use nava_typechecker::check;

fn lower_source(source: &str) -> (nava_ir::IrModule, String) {
    let parsed = parse(source);
    assert!(
        !parsed.diagnostics.has_errors(),
        "{}",
        parsed.diagnostics.render(source)
    );
    let typed = check(&parsed.program, source);
    assert!(
        !typed.diagnostics.has_errors(),
        "{}",
        typed.diagnostics.render(source)
    );
    let lowered = lower(&parsed.program, &typed.typed, source);
    assert!(
        !lowered.diagnostics.has_errors(),
        "{}",
        lowered.diagnostics.render(source)
    );
    (lowered.module, source.to_owned())
}

#[test]
fn lowers_a_simple_function_with_a_stack_slot() {
    let (module, _) = lower_source("function f(x: number): number { return x; }");
    let function = module.function("f").expect("function f");
    assert_eq!(function.return_ty, nava_ir::IrType::Number);
    assert_eq!(function.params.len(), 1);
    let entry = function.block(function.entry).expect("entry block");
    assert!(
        entry
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Alloc { .. }))
    );
    assert!(matches!(
        entry.terminator,
        Some(Terminator::Return(Some(_)))
    ));
}

#[test]
fn lowers_conditionals_into_basic_blocks() {
    let (module, _) =
        lower_source("function f(n: number): number { if (n <= 1) { return n; } return n - 1; }");
    let function = module.function("f").expect("function f");
    assert!(function.blocks.len() >= 4, "expected branch blocks");
    let entry = function.block(function.entry).expect("entry block");
    assert!(matches!(entry.terminator, Some(Terminator::CondBr { .. })));
    assert!(
        function
            .blocks
            .iter()
            .all(|block| block.terminator.is_some())
    );
}

#[test]
fn lowers_the_fibonacci_acceptance_program() {
    let source = include_str!("../../examples/fibonacci.ts");
    let (module, _) = lower_source(source);
    let fib = module.function("fibonacci").expect("fibonacci");
    assert_eq!(fib.params.len(), 1);
    assert!(
        fib.blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(Terminator::CondBr { .. })))
    );
    let calls: usize = fib
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter(|instruction| matches!(instruction, Instruction::Call { .. }))
        .count();
    assert_eq!(calls, 2, "fibonacci recurses twice");

    let main = module.function("main").expect("main");
    let call = main
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .find_map(|instruction| match instruction {
            Instruction::Call { callee, args, .. } => Some((callee.as_str(), args)),
            _ => None,
        })
        .expect("a call in main");
    assert_eq!(call.0, "fibonacci");
    assert_eq!(call.1, &[Value::Number(10.0)]);
}

#[test]
fn lowers_string_literals_as_module_globals() {
    let (module, _) = lower_source("console.log(\"hello\");");
    assert_eq!(module.globals.len(), 1);
    assert_eq!(module.globals[0].value, "hello");
}

#[test]
fn lower_supports_collection_literals() {
    let (module, _) = lower_source("let xs = [1, 2]; let p = { x: 1 };");
    assert!(module.functions[0].blocks.iter().any(|block| {
        block
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, nava_ir::Instruction::ArrayNew { .. }))
    }));
    assert!(module.functions[0].blocks.iter().any(|block| {
        block
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, nava_ir::Instruction::ObjectNew { .. }))
    }));
}

#[test]
fn reports_unknown_member_calls_instead_of_lowering_them() {
    let source = "let x = console; x.unknown();";
    let parsed = parse(source);
    let typed = check(&parsed.program, source);
    let lowered = lower(&parsed.program, &typed.typed, source);
    assert!(lowered.diagnostics.has_errors());
    let rendered = lowered.diagnostics.render(source);
    assert!(rendered.contains("unsupported call target"), "{rendered}");
}

#[test]
fn allocates_stack_slots_in_the_entry_block() {
    let source = "function f(n: number): number { if (n > 0) { let x = 1; return x; } return 0; }";
    let (module, _) = lower_source(source);
    let function = module.function("f").expect("f");
    let entry = function.block(function.entry).expect("entry");
    let allocs: usize = entry
        .instructions
        .iter()
        .filter(|instruction| matches!(instruction, Instruction::Alloc { .. }))
        .count();
    assert_eq!(allocs, 2, "parameter and local slots belong in entry");
    assert!(function.blocks[1..].iter().all(|block| {
        block
            .instructions
            .iter()
            .all(|instruction| !matches!(instruction, Instruction::Alloc { .. }))
    }));
}

#[test]
fn inferred_void_functions_have_no_call_result() {
    let source = "function p(x: number) { let y = x + 1; } p(1);";
    let parsed = parse(source);
    let typed = check(&parsed.program, source);
    let lowered = lower(&parsed.program, &typed.typed, source);
    let main = lowered.module.function("main").expect("main");
    let call = main
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .find_map(|instruction| match instruction {
            Instruction::Call { callee, result, .. } if callee == "p" => Some(result),
            _ => None,
        })
        .expect("call to p");
    assert!(call.is_none(), "a void call must not produce a value");
}

#[test]
fn loops_lower_to_verifiable_back_edges() {
    let (module, _) = lower_source(
        "function f(n: number): number { let i = 0; while (i < n) { i = i + 1; } return i; }",
    );
    assert!(nava_ir::verify(&module).is_empty());
    let function = module.function("f").expect("function f");
    let has_back_edge = function.blocks.iter().any(|block| match block.terminator {
        Some(Terminator::Br(target)) => target <= block.id,
        Some(Terminator::CondBr {
            then_block,
            else_block,
            ..
        }) => then_block <= block.id || else_block <= block.id,
        _ => false,
    });
    assert!(has_back_edge, "expected a branch back to the loop header");
    assert!(
        function
            .blocks
            .iter()
            .all(|block| block.terminator.is_some())
    );
}

#[test]
fn continue_keeps_do_and_infinite_loop_targets_alive() {
    for source in [
        "function f(): number { let i = 0; do { i = i + 1; continue; } while (i < 3); return i; }",
        "function f(): number { let i = 0; for (;;) { i = i + 1; continue; } return i; }",
        "function f(): number { let i = 0; while (true) { i = i + 1; continue; } return i; }",
    ] {
        let (module, _) = lower_source(source);
        assert!(nava_ir::verify(&module).is_empty(), "{source}");
    }
}

#[test]
fn break_and_continue_lower_to_loop_targets() {
    let (module, _) = lower_source(
        "function f(): number { let i = 0; while (i < 4) { i = i + 1; if (i == 2) continue; break; } return i; }",
    );
    assert!(nava_ir::verify(&module).is_empty());
    let function = module.function("f").expect("function f");
    let blocks = &function.blocks;
    let has_back_edge = blocks.iter().any(|block| match block.terminator {
        Some(Terminator::Br(target)) => target <= block.id,
        Some(Terminator::CondBr {
            then_block,
            else_block,
            ..
        }) => then_block <= block.id || else_block <= block.id,
        _ => false,
    });
    assert!(has_back_edge, "continue should create a back edge");
}

#[test]
fn optional_access_lowers_to_a_null_check() {
    let source = "interface Box { name: string; } let b: Box = { name: \"x\" }; let n = b?.name;";
    let (module, _) = lower_source(source);
    assert!(nava_ir::verify(&module).is_empty());
    let main = module.function("main").expect("main");
    assert!(
        main.blocks
            .iter()
            .any(|block| block.label.starts_with("optional."))
    );
}

#[test]
fn arrow_captures_and_value_aliases_are_diagnosed() {
    let source = "let base = 10; let add = (x: number): number => x + base;";
    let parsed = parse(source);
    let typed = check(&parsed.program, source);
    let lowered = lower(&parsed.program, &typed.typed, source);
    assert!(lowered.diagnostics.has_errors());

    let source = "let f = (x: number): number => x; let g = f;";
    let parsed = parse(source);
    let typed = check(&parsed.program, source);
    let lowered = lower(&parsed.program, &typed.typed, source);
    assert!(lowered.diagnostics.has_errors());
}

#[test]
fn arrow_initializers_lower_to_named_functions() {
    let (module, _) =
        lower_source("let add = (a: number, b: number): number => a + b; let n = add(1, 2);");
    assert!(nava_ir::verify(&module).is_empty());
    let function = module.function("add").expect("named arrow function");
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.return_ty, nava_ir::IrType::Number);
}

#[test]
fn ternary_in_declarations_and_statements_verifies() {
    let (module, _) = lower_source(
        "function f(n: number): number { let x = n > 0 ? 1 : 0; console.log(n > 0 ? \"a\" : \"b\"); return x; }",
    );
    assert!(nava_ir::verify(&module).is_empty());
}

#[test]
fn lowers_ternary_into_basic_blocks() {
    let (module, _) = lower_source("function f(n: number): number { return n > 0 ? 1 : -1; }");
    assert!(nava_ir::verify(&module).is_empty());
    let function = module.function("f").expect("function f");
    assert!(
        function
            .blocks
            .iter()
            .any(|block| block.label.starts_with("ternary."))
    );
    assert!(
        function
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Some(Terminator::CondBr { .. })))
    );
}

#[test]
fn lowers_power_and_compound_updates() {
    let (module, _) = lower_source("let x = 2; x += 3; x++; let p = x ** 2;");
    assert!(nava_ir::verify(&module).is_empty());
    let main = module.function("main").expect("main");
    let has_power = main
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .any(|instruction| {
            matches!(
                instruction,
                Instruction::Binary {
                    op: nava_ir::BinaryOp::Pow,
                    ..
                }
            )
        });
    assert!(has_power, "expected a power instruction");
}

#[test]
fn infinite_loops_that_always_return_do_not_need_a_fallthrough() {
    for source in [
        "function f(n: number): number { while (true) { return n; } }",
        "function f(n: number): number { for (;;) { return n; } }",
    ] {
        let (module, _) = lower_source(source);
        assert!(nava_ir::verify(&module).is_empty(), "{source}");
    }
}

#[test]
fn for_of_lowers_to_an_index_loop() {
    let (module, _) = lower_source("let xs = [1, 2]; for (let x of xs) { console.log(x); }");
    assert!(nava_ir::verify(&module).is_empty());
    let main = module.function("main").expect("main");
    assert!(main.blocks.iter().any(
        |block| matches!(block.terminator, Some(Terminator::Br(target)) if target <= block.id)
    ));
    assert!(
        main.blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .any(|instruction| matches!(instruction, Instruction::ArrayLength { .. }))
    );
}

#[test]
fn lowers_bigint_literals_and_integer_operations() {
    let (module, _) = lower_source("let a = 123n; let b = a + 1n; let c = a > b; let d = -a;");
    assert!(nava_ir::verify(&module).is_empty());
    let main = module.function("main").expect("main");
    let instructions: Vec<&Instruction> = main
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .collect();
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        Instruction::Store {
            value: Value::BigInt(123),
            ..
        }
    )));
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        Instruction::Alloc {
            ty: nava_ir::IrType::BigInt,
            ..
        }
    )));
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        Instruction::BigIntBinary {
            op: nava_ir::BinaryOp::Add,
            ..
        }
    )));
    assert!(instructions.iter().any(|instruction| matches!(
        instruction,
        Instruction::Unary {
            op: nava_ir::UnaryOp::NegBigInt,
            ..
        }
    )));
}

#[test]
fn optional_calls_lower_to_direct_calls() {
    let source = "function f(x: number): number { return x; } console.log(f?.(1));";
    let (module, _) = lower_source(source);
    assert!(nava_ir::verify(&module).is_empty());
    let main = module.function("main").expect("main");
    assert!(main.blocks.iter().any(
        |block| block.instructions.iter().any(|instruction| matches!(
            instruction,
            Instruction::Call { callee, .. } if callee == "f"
        ))
    ));
    assert!(
        !main
            .blocks
            .iter()
            .any(|block| block.label.starts_with("optional."))
    );
}

#[test]
fn every_lowered_module_verifies() {
    let sources = [
        "let x = 1 + 2;",
        "function f(x: number): number { if (x > 0) { return x; } else { return -x; } }",
        "function a(n: number): number { if (n) return 1; return a(n - 1); } a(3);",
    ];
    for source in sources {
        let (module, _) = lower_source(source);
        assert!(nava_ir::verify(&module).is_empty(), "failed for: {source}");
    }
}
