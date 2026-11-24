//! Per-function construction state: blocks, slots and value numbering.

use std::collections::{HashMap, HashSet};

use super::bindings::Captured;
use super::ir::{
    BasicBlock, BlockId, Instruction, IrFunction, IrParam, IrType, SlotId, Terminator, Value,
    ValueId,
};
use navascript_ast::Ident;

/// A `throw` in the current function targets the innermost enclosing catch.
#[derive(Debug, Clone)]
pub(super) struct TryContext {
    pub(super) value_slot: SlotId,
    pub(super) value_ty: IrType,
    pub(super) catch_block: BlockId,
    /// Set by `throw` so an unreachable catch block can be omitted.
    pub(super) catch_used: bool,
}

/// Targets for `break` and `continue` in the innermost active loop.
#[derive(Debug, Clone)]
pub(super) struct LoopContext {
    pub(super) break_target: BlockId,
    pub(super) continue_target: BlockId,
    /// Whether a `break` or `continue` jumps out; keeps target blocks reachable.
    pub(super) has_jump: bool,
}

pub(super) struct FunctionBuilder {
    pub(super) function: IrFunction,
    pub(super) current: Option<BlockId>,
    pub(super) next_value: ValueId,
    pub(super) next_slot: SlotId,
    pub(super) next_block: BlockId,
    pub(super) scopes: Vec<HashMap<String, SlotId>>,
    pub(super) loops: Vec<LoopContext>,
    pub(super) tries: Vec<TryContext>,
    /// Inner type for `async function` return wrapping (ADR-0029).
    pub(super) async_return: Option<IrType>,
    /// The `this` stack slot and its object type inside a class member.
    pub(super) this_slot: Option<SlotId>,
    pub(super) this_ty: Option<IrType>,
    /// Environment fields of this closure, keyed by captured name.
    pub(super) captures: HashMap<String, (u32, Captured)>,
    /// Locals and parameters whose bindings are shared cells.
    pub(super) boxed: HashMap<String, (SlotId, IrType, IrType)>,
    /// Names captured by arrows inside this function; these bindings are boxed.
    pub(super) captured_names: HashSet<String>,
    /// `this` captured from an enclosing method, if any.
    pub(super) this_capture: Option<(u32, IrType)>,
}

impl FunctionBuilder {
    pub(super) fn new(name: String, params: Vec<IrParam>, return_ty: IrType) -> Self {
        let entry = 0;
        let next_value = params.len() as ValueId;
        Self {
            function: IrFunction {
                name,
                params,
                return_ty,
                entry,
                blocks: vec![BasicBlock::new(entry, "entry")],
            },
            current: Some(entry),
            next_value,
            next_slot: 0,
            next_block: 1,
            scopes: vec![HashMap::new()],
            loops: Vec::new(),
            tries: Vec::new(),
            async_return: None,
            this_slot: None,
            this_ty: None,
            captures: HashMap::new(),
            boxed: HashMap::new(),
            captured_names: HashSet::new(),
            this_capture: None,
        }
    }

    pub(super) fn set_captured_names(&mut self, names: HashSet<String>) {
        self.captured_names = names;
    }

    pub(super) fn finish(self) -> IrFunction {
        self.function
    }

    pub(super) fn fresh_value(&mut self) -> ValueId {
        let value = self.next_value;
        self.next_value += 1;
        value
    }

    pub(super) fn new_block(&mut self, label: &str) -> BlockId {
        let id = self.next_block;
        self.next_block += 1;
        self.function
            .blocks
            .push(BasicBlock::new(id, format!("{label}.{id}")));
        id
    }

    pub(super) fn set_terminator(&mut self, block: BlockId, terminator: Terminator) {
        if let Some(block) = self.function.block_mut(block) {
            block.terminator = Some(terminator);
        }
    }

    pub(super) fn emit_in(&mut self, block: BlockId, instruction: Instruction) {
        if let Some(block) = self.function.block_mut(block) {
            block.instructions.push(instruction);
        }
    }

    pub(super) fn emit(&mut self, instruction: Instruction) {
        if let Some(id) = self.current {
            self.emit_in(id, instruction);
        }
    }

    /// Stack slots always live in the entry block so LLVM's `mem2reg` can
    /// promote them, regardless of where the variable was declared.
    pub(super) fn alloc(&mut self, ty: IrType) -> SlotId {
        let slot = self.next_slot;
        self.next_slot += 1;
        let entry = self.function.entry;
        self.emit_in(entry, Instruction::Alloc { slot, ty });
        slot
    }

    pub(super) fn store(&mut self, slot: SlotId, ty: IrType, value: Value) {
        self.emit(Instruction::Store { slot, ty, value });
    }

    pub(super) fn load(&mut self, slot: SlotId, ty: IrType) -> Value {
        let result = self.fresh_value();
        self.emit(Instruction::Load { result, slot, ty });
        Value::Local(result)
    }

    pub(super) fn declare(&mut self, name: &Ident, slot: SlotId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.name.clone(), slot);
        }
    }

    pub(super) fn slot_type(&self, slot: SlotId) -> Option<IrType> {
        self.function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .find_map(|instruction| match instruction {
                Instruction::Alloc { slot: found, ty } if *found == slot => Some(ty.clone()),
                _ => None,
            })
    }

    pub(super) fn lookup(&self, name: &str) -> Option<(SlotId, IrType)> {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return Some((slot, self.slot_type(slot).unwrap_or(IrType::Opaque)));
            }
        }
        None
    }

    pub(super) fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn leave_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }
}
