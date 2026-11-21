//! IR verifier: structural, dominance and type checks.

use std::collections::{HashMap, HashSet};
use std::fmt;

use nava_ast::Span;

use super::ir::{
    BinaryOp, BlockId, Instruction, IrFunction, IrModule, IrType, Terminator, Value, ValueId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyError {
    pub function: String,
    pub block: Option<BlockId>,
    pub message: String,
    pub span: Option<Span>,
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.block {
            Some(block) => write!(f, "{}:bb{}: {}", self.function, block, self.message),
            None => write!(f, "{}: {}", self.function, self.message),
        }
    }
}

/// Verify a module. Returns all errors found, not just the first.
pub fn verify(module: &IrModule) -> Vec<VerifyError> {
    let mut errors = Vec::new();
    let mut names = HashSet::new();
    for function in &module.functions {
        if !names.insert(function.name.clone()) {
            errors.push(error(function, None, "duplicate function name"));
        }
        verify_function(function, &mut errors);
    }
    let mut globals = HashSet::new();
    for global in &module.globals {
        if !globals.insert(global.id) {
            errors.push(VerifyError {
                function: "<module>".to_owned(),
                block: None,
                message: format!("duplicate global @{}", global.id),
                span: None,
            });
        }
    }
    for function in &module.functions {
        for block in &function.blocks {
            for value in block
                .instructions
                .iter()
                .flat_map(values_in_instruction)
                .chain(
                    block
                        .terminator
                        .as_ref()
                        .into_iter()
                        .flat_map(values_in_terminator),
                )
            {
                if let Value::String(id) = value {
                    if !globals.contains(id) {
                        errors.push(error(
                            function,
                            Some(block.id),
                            format!("unknown string global @{id}"),
                        ));
                    }
                }
            }
        }
    }
    errors
}

fn values_in_instruction(instruction: &Instruction) -> Vec<&Value> {
    match instruction {
        Instruction::Store { value, .. } | Instruction::ToBool { value, .. } => vec![value],
        Instruction::Binary { lhs, rhs, .. }
        | Instruction::StringConcat {
            left: lhs,
            right: rhs,
            ..
        }
        | Instruction::StringEquals {
            left: lhs,
            right: rhs,
            ..
        } => vec![lhs, rhs],
        Instruction::Unary { operand, .. }
        | Instruction::StringLength { value: operand, .. }
        | Instruction::NumberToString { value: operand, .. } => vec![operand],
        Instruction::StringCharAt { value, index, .. } => vec![value, index],
        Instruction::Call { args, .. } => args.iter().collect(),
        Instruction::ArrayGet { array, index, .. } => vec![array, index],
        Instruction::ArraySet {
            array,
            index,
            value,
            ..
        } => vec![array, index, value],
        Instruction::ArrayLength { array, .. } => vec![array],
        Instruction::ObjectGet { object, .. } => vec![object],
        Instruction::ObjectSet { object, value, .. } => vec![object, value],
        Instruction::Alloc { .. }
        | Instruction::Load { .. }
        | Instruction::ArrayNew { .. }
        | Instruction::ObjectNew { .. } => Vec::new(),
    }
}

fn values_in_terminator(terminator: &Terminator) -> Vec<&Value> {
    match terminator {
        Terminator::Return(Some(value)) => vec![value],
        Terminator::CondBr { cond, .. } => vec![cond],
        _ => Vec::new(),
    }
}

fn error(function: &IrFunction, block: Option<BlockId>, message: impl Into<String>) -> VerifyError {
    VerifyError {
        function: function.name.clone(),
        block,
        message: message.into(),
        span: None,
    }
}

fn verify_function(function: &IrFunction, errors: &mut Vec<VerifyError>) {
    let mut block_ids = HashSet::new();
    let mut labels = HashSet::new();
    for block in &function.blocks {
        if !block_ids.insert(block.id) {
            errors.push(error(function, Some(block.id), "duplicate block id"));
        }
        if !labels.insert(block.label.clone()) {
            errors.push(error(function, Some(block.id), "duplicate block label"));
        }
    }
    if function.block(function.entry).is_none() {
        errors.push(error(function, None, "missing entry block"));
        return;
    }

    let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for block in &function.blocks {
        let mut succ = Vec::new();
        match &block.terminator {
            None => errors.push(error(function, Some(block.id), "block has no terminator")),
            Some(Terminator::Return(_)) => {}
            Some(Terminator::Br(target)) => succ.push(*target),
            Some(Terminator::CondBr {
                then_block,
                else_block,
                ..
            }) => {
                succ.push(*then_block);
                succ.push(*else_block);
            }
        }
        for target in &succ {
            if function.block(*target).is_none() {
                errors.push(error(
                    function,
                    Some(block.id),
                    format!("branch to unknown block bb{target}"),
                ));
            }
            predecessors.entry(*target).or_default().push(block.id);
        }
    }

    let dominators = compute_dominators(function, &predecessors);
    let mut defs: HashMap<ValueId, (IrType, BlockId)> = HashMap::new();
    for (index, param) in function.params.iter().enumerate() {
        defs.insert(index as ValueId, (param.ty.clone(), function.entry));
    }
    let mut slot_types: HashMap<u32, (IrType, BlockId)> = HashMap::new();

    // First pass: collect definitions so forward references can be detected.
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Alloc { slot, ty } => {
                    if slot_types.insert(*slot, (ty.clone(), block.id)).is_some() {
                        errors.push(error(function, Some(block.id), "duplicate stack slot"));
                    }
                }
                Instruction::Load { result, ty, .. } => {
                    if defs.insert(*result, (ty.clone(), block.id)).is_some() {
                        errors.push(error(function, Some(block.id), "duplicate value id"));
                    }
                }
                Instruction::Binary { result, .. }
                | Instruction::Unary { result, .. }
                | Instruction::ToBool { result, .. }
                | Instruction::StringConcat { result, .. }
                | Instruction::StringEquals { result, .. }
                | Instruction::StringLength { result, .. }
                | Instruction::NumberToString { result, .. }
                | Instruction::StringCharAt { result, .. }
                | Instruction::ArrayNew { result, .. }
                | Instruction::ArrayGet { result, .. }
                | Instruction::ArrayLength { result, .. }
                | Instruction::ObjectNew { result, .. }
                | Instruction::ObjectGet { result, .. } => {
                    if defs
                        .insert(*result, (instruction_type(instruction), block.id))
                        .is_some()
                    {
                        errors.push(error(function, Some(block.id), "duplicate value id"));
                    }
                }
                Instruction::Call {
                    result: Some(result),
                    result_ty,
                    ..
                } if defs
                    .insert(*result, (result_ty.clone(), block.id))
                    .is_some() =>
                {
                    errors.push(error(function, Some(block.id), "duplicate value id"));
                }
                _ => {}
            }
        }
    }

    // Second pass: check uses are dominated by their definitions and types agree.
    for block in &function.blocks {
        let mut available_values: HashSet<ValueId> = HashSet::new();
        let mut available_slots: HashSet<u32> = HashSet::new();
        // Definitions in the current block are inserted as we walk its
        // instructions, so use-before-def inside a block is rejected too.
        for (id, (_, defined_in)) in &defs {
            if *defined_in != block.id && dominators[&block.id].contains(defined_in) {
                available_values.insert(*id);
            }
        }
        for (slot, (_, defined_in)) in &slot_types {
            if *defined_in != block.id && dominators[&block.id].contains(defined_in) {
                available_slots.insert(*slot);
            }
        }

        for instruction in &block.instructions {
            match instruction {
                Instruction::Alloc { slot, .. } => {
                    available_slots.insert(*slot);
                }
                Instruction::Store { slot, ty, value } => {
                    check_slot(
                        slot,
                        ty,
                        &available_slots,
                        &slot_types,
                        function,
                        block.id,
                        errors,
                    );
                    check_value(value, &available_values, function, block.id, errors);
                }
                Instruction::Load { result, slot, ty } => {
                    check_slot(
                        slot,
                        ty,
                        &available_slots,
                        &slot_types,
                        function,
                        block.id,
                        errors,
                    );
                    available_values.insert(*result);
                }
                Instruction::Binary {
                    result,
                    op,
                    lhs,
                    rhs,
                } => {
                    check_value(lhs, &available_values, function, block.id, errors);
                    check_value(rhs, &available_values, function, block.id, errors);
                    let lhs_ty = value_type(lhs, &defs);
                    let rhs_ty = value_type(rhs, &defs);
                    if *op == BinaryOp::And || *op == BinaryOp::Or {
                        require(&lhs_ty, IrType::Boolean, function, block.id, errors);
                        require(&rhs_ty, IrType::Boolean, function, block.id, errors);
                    } else if op.is_arithmetic() {
                        require(&lhs_ty, IrType::Number, function, block.id, errors);
                        require(&rhs_ty, IrType::Number, function, block.id, errors);
                    }
                    available_values.insert(*result);
                }
                Instruction::Unary {
                    result,
                    op,
                    operand,
                } => {
                    check_value(operand, &available_values, function, block.id, errors);
                    let operand_ty = value_type(operand, &defs);
                    match op {
                        super::ir::UnaryOp::Neg => {
                            require(&operand_ty, IrType::Number, function, block.id, errors)
                        }
                        super::ir::UnaryOp::Not => {
                            require(&operand_ty, IrType::Boolean, function, block.id, errors)
                        }
                    }
                    available_values.insert(*result);
                }
                Instruction::Call { result, args, .. } => {
                    for arg in args {
                        check_value(arg, &available_values, function, block.id, errors);
                    }
                    if let Some(result) = result {
                        available_values.insert(*result);
                    }
                }
                Instruction::ToBool { result, value }
                | Instruction::StringLength { result, value }
                | Instruction::NumberToString { result, value } => {
                    check_value(value, &available_values, function, block.id, errors);
                    available_values.insert(*result);
                }
                Instruction::StringConcat {
                    result,
                    left,
                    right,
                }
                | Instruction::StringEquals {
                    result,
                    left,
                    right,
                } => {
                    check_value(left, &available_values, function, block.id, errors);
                    check_value(right, &available_values, function, block.id, errors);
                    available_values.insert(*result);
                }
                Instruction::StringCharAt {
                    result,
                    value,
                    index,
                }
                | Instruction::ArrayGet {
                    result,
                    array: value,
                    index,
                    ..
                } => {
                    check_value(value, &available_values, function, block.id, errors);
                    check_value(index, &available_values, function, block.id, errors);
                    available_values.insert(*result);
                }
                Instruction::ArrayNew { result, .. } | Instruction::ObjectNew { result, .. } => {
                    available_values.insert(*result);
                }
                Instruction::ArraySet {
                    array,
                    index,
                    value,
                    ..
                } => {
                    check_value(array, &available_values, function, block.id, errors);
                    check_value(index, &available_values, function, block.id, errors);
                    check_value(value, &available_values, function, block.id, errors);
                }
                Instruction::ArrayLength { result, array } => {
                    check_value(array, &available_values, function, block.id, errors);
                    available_values.insert(*result);
                }
                Instruction::ObjectGet { result, object, .. } => {
                    check_value(object, &available_values, function, block.id, errors);
                    available_values.insert(*result);
                }
                Instruction::ObjectSet { object, value, .. } => {
                    check_value(object, &available_values, function, block.id, errors);
                    check_value(value, &available_values, function, block.id, errors);
                }
            }
        }

        if let Some(terminator) = &block.terminator {
            match terminator {
                Terminator::Return(value) => {
                    if let Some(value) = value {
                        check_value(value, &available_values, function, block.id, errors);
                        let actual = value_type(value, &defs);
                        if function.return_ty != IrType::Opaque
                            && actual != IrType::Opaque
                            && actual != function.return_ty
                        {
                            errors.push(error(
                                function,
                                Some(block.id),
                                format!(
                                    "returns '{}' but function returns '{}'",
                                    actual.display(),
                                    function.return_ty.display()
                                ),
                            ));
                        }
                    } else if function.return_ty != IrType::Void {
                        errors.push(error(
                            function,
                            Some(block.id),
                            "missing return value in non-void function",
                        ));
                    }
                }
                Terminator::Br(_) => {}
                Terminator::CondBr { cond, .. } => {
                    check_value(cond, &available_values, function, block.id, errors);
                    require(
                        &value_type(cond, &defs),
                        IrType::Boolean,
                        function,
                        block.id,
                        errors,
                    );
                }
            }
        }
    }
}

fn require(
    actual: &IrType,
    expected: IrType,
    function: &IrFunction,
    block: BlockId,
    errors: &mut Vec<VerifyError>,
) {
    if *actual != IrType::Opaque && *actual != expected {
        errors.push(error(
            function,
            Some(block),
            format!(
                "expected '{}', found '{}'",
                expected.display(),
                actual.display()
            ),
        ));
    }
}

fn check_slot(
    slot: &u32,
    ty: &IrType,
    available: &HashSet<u32>,
    slot_types: &HashMap<u32, (IrType, BlockId)>,
    function: &IrFunction,
    block: BlockId,
    errors: &mut Vec<VerifyError>,
) {
    if !available.contains(slot) {
        errors.push(error(
            function,
            Some(block),
            format!("stack slot ${slot} is not defined here"),
        ));
    }
    if let Some((declared, _)) = slot_types.get(slot) {
        if declared != ty {
            errors.push(error(
                function,
                Some(block),
                format!(
                    "slot ${slot} has type '{}', used as '{}'",
                    declared.display(),
                    ty.display()
                ),
            ));
        }
    }
}

fn check_value(
    value: &Value,
    available: &HashSet<ValueId>,
    function: &IrFunction,
    block: BlockId,
    errors: &mut Vec<VerifyError>,
) {
    match value {
        Value::Local(id) => {
            if !available.contains(id) {
                errors.push(error(
                    function,
                    Some(block),
                    format!("value %{id} is not defined here"),
                ));
            }
        }
        Value::Param(index) if *index as usize >= function.params.len() => {
            errors.push(error(
                function,
                Some(block),
                format!("parameter %{index} is out of range"),
            ));
        }
        _ => {}
    }
}

fn value_type(value: &Value, defs: &HashMap<ValueId, (IrType, BlockId)>) -> IrType {
    match value {
        Value::Number(_) => IrType::Number,
        Value::Bool(_) => IrType::Boolean,
        Value::String(_) => IrType::String,
        Value::Null => IrType::Opaque,
        Value::Param(index) | Value::Local(index) => defs
            .get(index)
            .map(|(ty, _)| ty.clone())
            .unwrap_or(IrType::Opaque),
    }
}

fn instruction_type(instruction: &Instruction) -> IrType {
    match instruction {
        Instruction::Binary { op, .. } => {
            if op.is_comparison() {
                IrType::Boolean
            } else {
                IrType::Number
            }
        }
        Instruction::Unary { op, .. } => match op {
            super::ir::UnaryOp::Neg => IrType::Number,
            super::ir::UnaryOp::Not => IrType::Boolean,
        },
        Instruction::Load { ty, .. } => ty.clone(),
        Instruction::Call { result_ty, .. } => result_ty.clone(),
        Instruction::ToBool { .. } | Instruction::StringEquals { .. } => IrType::Boolean,
        Instruction::StringLength { .. } | Instruction::ArrayLength { .. } => IrType::Number,
        Instruction::StringConcat { .. }
        | Instruction::StringCharAt { .. }
        | Instruction::NumberToString { .. } => IrType::String,
        Instruction::ArrayNew { element_ty, .. } => IrType::Array(Box::new(element_ty.clone())),
        Instruction::ArrayGet { element_ty, .. } => element_ty.clone(),
        Instruction::ObjectNew { fields, .. } => IrType::Object(fields.clone()),
        Instruction::ObjectGet { field_ty, .. } => field_ty.clone(),
        _ => IrType::Opaque,
    }
}

fn compute_dominators(
    function: &IrFunction,
    predecessors: &HashMap<BlockId, Vec<BlockId>>,
) -> HashMap<BlockId, HashSet<BlockId>> {
    let all: HashSet<BlockId> = function.blocks.iter().map(|block| block.id).collect();
    let mut dominators: HashMap<BlockId, HashSet<BlockId>> = function
        .blocks
        .iter()
        .map(|block| (block.id, all.clone()))
        .collect();
    dominators.insert(function.entry, HashSet::from([function.entry]));

    loop {
        let mut changed = false;
        for block in &function.blocks {
            if block.id == function.entry {
                continue;
            }
            let preds = predecessors.get(&block.id).cloned().unwrap_or_default();
            let mut new_set = match preds.first() {
                Some(first) => dominators.get(first).cloned().unwrap_or_default(),
                None => HashSet::new(),
            };
            for pred in preds.iter().skip(1) {
                if let Some(set) = dominators.get(pred) {
                    new_set.retain(|id| set.contains(id));
                }
            }
            new_set.insert(block.id);
            if dominators.get(&block.id) != Some(&new_set) {
                dominators.insert(block.id, new_set);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    dominators
}
