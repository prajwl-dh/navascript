//! The type checker: one `check` seam hiding scopes, inference and checking.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::classes::ClassInfo;

use nava_ast::{FileId, Item, ItemKind, Program, Span};
use nava_diagnostics::{Diagnostic, Diagnostics, LineIndex};
use nava_symbol::{Symbol, SymbolKind, SymbolTable, Type};

use super::types::AliasTable;

/// Result of checking: diagnostics plus the type information IR lowering needs.
#[derive(Debug, Clone, PartialEq)]
pub struct CheckOutput {
    pub diagnostics: Diagnostics,
    pub typed: TypedProgram,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    /// Expression types keyed by the full source span. The start byte alone is
    /// ambiguous: an identifier and a member expression can share it.
    pub types: HashMap<Span, Type>,
    pub symbols: SymbolTable,
    /// Generic parameter names for functions, interfaces and aliases.
    pub generic_params: HashMap<String, Vec<String>>,
    /// Resolved type arguments for each call expression, in parameter order.
    /// IR lowering uses this to select a monomorphization.
    pub call_instantiations: HashMap<Span, Vec<Type>>,
    /// Checked class shapes, including inherited members.
    pub classes: HashMap<String, ClassInfo>,
    /// Mangled method target for each `obj.method(...)` call expression.
    pub method_calls: HashMap<Span, String>,
    /// Base constructor target for each `super(args)` call.
    pub super_calls: HashMap<Span, String>,
}

pub fn check(program: &Program, source: &str) -> CheckOutput {
    let mut checker = Checker::new(program, source);
    checker.seed_builtins();
    checker.collect_aliases();
    checker.check_alias_cycles();
    checker.collect_classes();
    checker.collect_declarations();
    checker.check_imports();
    for item in &program.items {
        checker.check_item(item);
    }
    CheckOutput {
        diagnostics: Diagnostics::new(checker.diagnostics),
        typed: TypedProgram {
            types: checker.types,
            symbols: checker.symbols,
            generic_params: checker.generic_params,
            call_instantiations: checker.call_instantiations,
            classes: checker.classes,
            method_calls: checker.method_calls,
            super_calls: checker.super_calls,
        },
    }
}

pub(super) struct Checker<'a> {
    pub(super) program: &'a Program,
    pub(super) lines: LineIndex,
    pub(super) symbols: SymbolTable,
    pub(super) aliases: AliasTable,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) types: HashMap<Span, Type>,
    pub(super) loop_depth: u32,
    pub(super) generic_params: HashMap<String, Vec<String>>,
    pub(super) call_instantiations: HashMap<Span, Vec<Type>>,
    pub(super) classes: HashMap<String, ClassInfo>,
    pub(super) class_resolved: HashSet<String>,
    pub(super) method_calls: HashMap<Span, String>,
    /// The class whose body is currently being checked (for `super(...)`).
    pub(super) current_class: Option<String>,
    pub(super) super_calls: HashMap<Span, String>,
}

impl<'a> Checker<'a> {
    pub(super) fn new(program: &'a Program, source: &str) -> Self {
        Self {
            program,
            lines: LineIndex::new(source),
            symbols: SymbolTable::new(),
            aliases: AliasTable::new(),
            diagnostics: Vec::new(),
            types: HashMap::new(),
            loop_depth: 0,
            generic_params: HashMap::new(),
            call_instantiations: HashMap::new(),
            classes: HashMap::new(),
            class_resolved: HashSet::new(),
            method_calls: HashMap::new(),
            current_class: None,
            super_calls: HashMap::new(),
        }
    }

    /// Minimal built-in surface needed by the fibonacci acceptance program.
    /// The real stdlib arrives in Phase 5.
    pub(super) fn seed_builtins(&mut self) {
        let mut console = BTreeMap::new();
        console.insert(
            "log".to_owned(),
            Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Void),
            },
        );
        self.symbols.insert(Symbol {
            name: "console".to_owned(),
            kind: SymbolKind::Variable,
            ty: Type::Object(console),
            span: Span::new(FileId(0), 0, 0),
        });
    }

    pub(super) fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(message, span, &self.lines));
    }

    /// Imported names must be exported by some module. Exported classes and
    /// type declarations live in separate registries, so this checks the
    /// `export` wrappers rather than only the symbol table.
    pub(super) fn check_imports(&mut self) {
        let exported: std::collections::HashSet<String> = self
            .program
            .items
            .iter()
            .filter(|item| matches!(item.kind, ItemKind::Export(_)))
            .filter_map(|item| item.kind.declared_name().map(str::to_owned))
            .collect();
        let mut errors: Vec<(Span, String)> = Vec::new();
        for item in &self.program.items {
            if let ItemKind::Import {
                names, source_span, ..
            } = item.kind.unwrap_export()
            {
                if names.is_empty() {
                    errors.push((*source_span, "import list is empty".to_owned()));
                }
                for name in names {
                    if !exported.contains(&name.name) {
                        errors.push((
                            name.span,
                            format!("imported name '{}' is not exported", name.name),
                        ));
                    }
                }
            }
        }
        for (span, message) in errors {
            self.error(message, span);
        }
    }

    pub(super) fn check_item(&mut self, item: &Item) {
        match item.kind.unwrap_export() {
            ItemKind::Variable(decl) => self.check_variable(decl, true),
            ItemKind::Function(decl) => self.check_function(decl),
            ItemKind::Class(decl) => self.check_class(decl),
            ItemKind::Expr(expr) => {
                self.infer_expr(expr);
            }
            ItemKind::Stmt(stmt) => self.check_stmt(stmt, None),
            ItemKind::TypeAlias(_)
            | ItemKind::Interface(_)
            | ItemKind::Import { .. }
            | ItemKind::Export(_) => {}
        }
    }
}
