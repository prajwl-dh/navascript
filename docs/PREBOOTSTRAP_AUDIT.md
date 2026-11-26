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

The chosen "full breadth" scope named GC, tagged unions, per-file namespaces
and `any`. Phase 8.1 implemented most of them; the table below records what
remains partial rather than overclaiming:

| Gap | Phase 8.1 status | Remaining limitation |
| --- | --- | --- |
| Garbage collection | Conservative mark-sweep with block headers and explicit `collect()` (ADR-0034). | Not automatic/generational; conservative scans may retain garbage and register scanning is AArch64-specific. |
| Tagged unions | Mixed unions now use tagged values with checked conversions. | No tag-based narrowing; nullable primitives remain diagnosed. |
| Per-file namespaces | Diagnostics name the originating file via `SourceMap`. | Symbol scoping is still one global namespace. |
| `any` | `Type::Any` / `IrType::Dynamic` with explicit boxed conversions. | No automatic coercion or narrowing; `unknown` remains internal only. |

Phase 8.1 closed most of these gaps; the remaining limitations above are
recorded in `COMPATIBILITY.md` §6. This audit does **not** claim a 100%
TypeScript-compatible or fully self-hosted compiler;
it claims that every supported feature is tested and every unsupported feature
is diagnosed rather than miscompiled.

## 4. Verification evidence

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- `cargo test --workspace`: 299 tests pass.
- Python harness suites: 8 compatibility + 4 benchmark tests pass.
- `python3 scripts/compat.py --check`: report byte-identical (19 executed cases).
- `python3 scripts/benchmark.py --check`: deterministic benchmark facts match.
- Native stdlib program produces the expected stdout and writes the expected file.
