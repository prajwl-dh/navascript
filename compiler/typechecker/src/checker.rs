//! The type checker: one `check` seam hiding scopes, inference and checking.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::classes::ClassInfo;

use navascript_ast::{FileId, Item, ItemKind, Program, Span};
use navascript_diagnostics::{Diagnostic, Diagnostics, LineIndex};
use navascript_symbol::{Symbol, SymbolKind, SymbolTable, Type};

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
    /// Catch types of enclosing `try` statements in the current function. A
    /// `throw` in a different function (e.g. an arrow body) has no handler.
    pub(super) catch_types: Vec<Type>,
    /// Depth of `try`/`catch`/`finally` bodies, where abrupt control flow is
    /// not supported yet (ADR-0028).
    pub(super) protected_depth: u32,
    /// Depth of catch/finally bodies, where `throw` is unsupported because it
    /// would bypass the finally block (ADR-0028).
    pub(super) throw_forbidden: u32,
    /// `Some(false)` inside a non-async function, `None` at program scope where
    /// top-level `await` is allowed (ADR-0029).
    pub(super) current_function_async: Option<bool>,
    pub(super) generic_params: HashMap<String, Vec<String>>,
    pub(super) call_instantiations: HashMap<Span, Vec<Type>>,
    pub(super) classes: HashMap<String, ClassInfo>,
    pub(super) class_resolved: HashSet<String>,
    /// Classes currently being resolved, to diagnose inheritance cycles.
    pub(super) class_resolving: HashSet<String>,
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
            catch_types: Vec::new(),
            protected_depth: 0,
            throw_forbidden: 0,
            current_function_async: None,
            generic_params: HashMap::new(),
            call_instantiations: HashMap::new(),
            classes: HashMap::new(),
            class_resolved: HashSet::new(),
            class_resolving: HashSet::new(),
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

        // Stdlib builtins required to implement the compiler in NavaScript.
        // Each name maps to a `navascript_<name>` runtime function in codegen.
        let number = Type::Number;
        let string = Type::String;
        let string_array = Type::Array(Box::new(Type::String));
        let number_array = Type::Array(Box::new(Type::Number));
        let builtin = |name: &str, params: Vec<Type>, ret: Type| Symbol {
            name: name.to_owned(),
            kind: SymbolKind::Function,
            ty: Type::Function {
                params,
                ret: Box::new(ret),
            },
            span: Span::new(FileId(0), 0, 0),
        };
        for symbol in [
            builtin("read_file", vec![string.clone()], string.clone()),
            builtin(
                "write_file",
                vec![string.clone(), string.clone()],
                Type::Void,
            ),
            builtin("get_argv", vec![], string_array.clone()),
            builtin(
                "str_slice",
                vec![string.clone(), number.clone(), number.clone()],
                string.clone(),
            ),
            builtin(
                "str_index_of",
                vec![string.clone(), string.clone()],
                number.clone(),
            ),
            builtin(
                "str_char_code_at",
                vec![string.clone(), number.clone()],
                number.clone(),
            ),
            builtin(
                "array_push_number",
                vec![number_array.clone(), number.clone()],
                number_array,
            ),
            builtin(
                "array_push_string",
                vec![string_array.clone(), string.clone()],
                string_array,
            ),
            // Tagged dynamic values (`any` and mixed unions).
            builtin("to_any_number", vec![number.clone()], Type::Any),
            builtin("to_any_bigint", vec![Type::BigInt], Type::Any),
            builtin("to_any_bool", vec![Type::Boolean], Type::Any),
            builtin("to_any_string", vec![string.clone()], Type::Any),
            builtin("to_any_null", vec![], Type::Any),
            builtin("as_number", vec![Type::Any], number.clone()),
            builtin("as_bigint", vec![Type::Any], Type::BigInt),
            builtin("as_bool", vec![Type::Any], Type::Boolean),
            builtin("as_string", vec![Type::Any], string.clone()),
            builtin("tag_of", vec![Type::Any], number.clone()),
            builtin("collect", vec![], Type::Void),
            builtin("live_bytes", vec![], Type::Number),
        ] {
            self.symbols.insert(symbol);
        }
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
