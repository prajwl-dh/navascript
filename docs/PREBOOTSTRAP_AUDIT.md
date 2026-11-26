# NavaScript — Pre-bootstrap codebase audit (2026-11-25)

This document records the whole-codebase audit performed before Phase 9
(Compiler Rewrite in NavaScript). Four read-only audits covered the
language/type system, runtime/stdlib, driver/diagnostics, and tests/known gaps;
their findings were cross-checked, fixed where provable, and then independently
reviewed on the Standards and Spec axes.

The guiding rule for this increment was: **never claim a capability that is not
backed by a passing test, and never leave a known miscompile silent.** An
unsupported construct must be diagnosed, not compiled into wrong behaviour.

## 1. What was fixed

### Correctness bugs
- `NumberToString` IR results were typed `boolean` while codegen emits a string
  pointer (`compiler/ir/src/results.rs`).
- `navascript build <file-without-extension>` could link the executable over
  its own source; the driver now refuses an output that resolves to the input
  (`compiler/driver/src/cli.rs`).
- A user-defined `function main()` collided with the process entry shim. The
  synthetic entry is now `__navascript_entry`, user `main` gets a distinct LLVM
  symbol, and all generated names use a reserved prefix that the checker
  diagnoses in user code (`ast`, `typechecker`, `ir`, `codegen`).
- LLVM IR object-name globals were emitted in hash order, making output
  non-reproducible; they are now sorted (`compiler/codegen/src/module.rs`).
- `console.log(boolean)` emitted an `i1` argument against an `i32` C prototype;
  the call now widens with `zext` (`compiler/codegen/src/instructions.rs`).
- The IR verifier now checks direct/indirect call arity and argument/result
  types, `IsNull` reference operands, void returns with values, and entry-block
  order; call checks live in private `ir/verify_calls.rs`.
- Circular inheritance now produces a diagnostic instead of unbounded
  recursion (`compiler/typechecker/src/classes.rs`).
- Radix literals with no digits (`0x`) now produce a lex diagnostic instead of
  silently decoding to `NaN` (`compiler/lexer/src/scanner/number.rs`).
- String concatenation with unsupported types is diagnosed instead of lowering
  a pointer through the number-to-string coercion (`typechecker/expressions.rs`).
- String ordering is now lowered through a real lexicographic runtime helper
  (`str_compare`) instead of an LLVM pointer comparison.
- Runtime allocation-size overflow, null pointer access, large-double-to-integer
  UB in number formatting, and computed fractional/NaN indices are now checked
  (`runtime/memory`, `runtime/arrays`, `runtime/objects`, `runtime/io`).
- `NaN`/`Infinity` now format as JavaScript-style `NaN`/`Infinity`.

### Self-hosting capabilities
- Typed stdlib builtins: `read_file`, `write_file`, `get_argv`, `str_slice`,
  `str_index_of`, `str_char_code_at`, `str_compare`, `array_push_number`,
  `array_push_string` (`compiler/typechecker/src/checker.rs`,
  `runtime/io/io.c`, `compiler/codegen/src/runtime.rs`).
- The runtime entry bridge passes OS `argc`/`argv` into NavaScript.
- The arena grows in linked chunks on demand instead of failing at a fixed
  16 MiB; pointers remain stable for the process lifetime.
- `examples/self_host_stdlib.ns` and a native test exercise every builtin,
  including reading a file, argument access, string helpers, array growth and
  writing a file.

### Test/process hardening
- Regression tests for the source-overwrite guard, user `main`, generated-name
  collisions, malformed radix literals, circular inheritance, reserved names,
  unsupported concatenation, and the stdlib path.
- Release-only native tests skip cleanly when LLVM `opt` is unavailable.
- Compatibility report wording and benchmark `--check` now verify the
  deterministic facts they actually claim.

## 2. Self-hosting readiness

Available now: strings (concat, length, indexing, comparison, slicing, search,
char codes), arrays and objects with typed access, classes and inheritance,
closures/higher-order functions, loops, recursion, modules, same-function
exceptions, synchronous promises, file I/O, process arguments, growable
program-lifetime allocation, and deterministic native code generation.

A lexer and parser can be written in this subset today. The compiler driver
would use `read_file`/`write_file`/`get_argv` instead of Rust I/O.

## 3. Remaining long-term gaps (deliberately NOT claimed in this increment)

The chosen "full breadth" scope also named GC, tagged unions, per-file
namespaces and `any`. They are **not implemented**, and claiming otherwise
would be false. Each needs a representation change rather than an incremental
feature, and shipping a partial version would break the "no silent
miscompiles" rule:

| Gap | Why it is not a small increment | Current safe behaviour |
| --- | --- | --- |
| Garbage collection | Runtime values are unboxed and have no type tags; a tracing collector needs tags, root tracking and write barriers across strings, arrays, objects, closures and promises. | Growable bump arena; memory is reclaimed at process exit. Fatal, documented (ADR-0005/ADR-0033). |
| Tagged unions / discriminated narrowing | Mixed-representation unions have no runtime tag or common layout, so narrowing cannot be sound without redesigning `Type`, `IrType` and the object ABI. | Mixed unions are diagnosed, never silently miscompiled (ADR-0025). |
| Per-file module namespaces | Imports are concatenated into one program; proper namespaces require file-aware symbol tables, `Span::FileId` in diagnostics, and per-file import binding. | One module-wide namespace; diagnostics refer to concatenated source (ADR-0026). |
| `any` / `unknown` | A source-level `any` needs a tagged runtime value that can hold numbers, strings, references and functions; the checker's internal `Type::Unknown` is only an inference fallback and cannot be lowered generically. | `unknown` is internal only; heterogeneous values use interfaces/classes/arrays. |

These gaps are recorded in `COMPATIBILITY.md` §6 and must be addressed as
explicit phases before they are used by ported compiler components. This audit
does **not** claim a 100% TypeScript-compatible or fully self-hosted compiler;
it claims that every supported feature is tested and every unsupported feature
is diagnosed rather than miscompiled.

## 4. Verification evidence

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo test --workspace`: 299 tests pass.
- Python harness suites: 8 compatibility + 4 benchmark tests pass.
- `python3 scripts/compat.py --check`: report byte-identical (19 executed cases).
- `python3 scripts/benchmark.py --check`: deterministic benchmark facts match.
- Native stdlib program produces the expected stdout and writes the expected file.
