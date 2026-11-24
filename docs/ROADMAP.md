# NavaScript — Roadmap

This document defines the 12 milestones. Every milestone lists objectives,
implementation tasks, tests, acceptance criteria, and known limitations. Status
is tracked in `docs/PROGRESS.md`; this file is the stable definition of *what*.

**Guiding principles**
- Build one milestone at a time; each must produce something executable or
  independently testable.
- Correctness and modularity beat speed of code generation.
- Keep TypeScript compatibility, native compilation, and self-hosting as three
  *separate* goals. Progress on one must not be claimed as progress on another.

**Phase exit protocol**
Before a phase is marked complete in `PROGRESS.md`:

1. Every task has a passing test or a recorded, accepted deviation.
2. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
   warnings`, and `cargo test --workspace` pass.
3. `PROGRESS.md` records the evidence (commands + results) and a work-log entry.
4. The phase's changes are committed.
5. A summary of work, test results and remaining tasks is given before the next
   phase starts.

**Phase prerequisites and exit evidence**

| Phase | Depends on | Exit evidence |
| --- | --- | --- |
| 0 Foundation | macOS toolchain | CLI builds; workspace tests pass |
| 1 Lexer/Parser | 0 | `parse(&str)` yields AST + diagnostics; tests at the seam |
| 2 Type system | 1 | negative type tests fail as expected |
| 3 IR | 2 | IR verifier accepts valid, rejects malformed IR |
| 4 Native backend | 3 | fibonacci(10) prints 55 natively, no Node.js |
| 5 Runtime | 4 | nontrivial app runs without Node.js |
| 6 Language expansion | 5 | expanded compatibility suite passes |
| 7 Compatibility | 6 | reproducible compatibility report committed |
| 8 Optimization | 7 | benchmarks show measured improvement |
| 9 Self-hosting | 8 | substantial compiler source in NavaScript; differential tests pass |
| 10 Bootstrap | 9 | stage 2/3 compile source without Rust |
| 11 Production | 10 | distribution tests on two or more targets |

---

## Phase 0 — Project Foundation

**Objectives**
- Establish a working Rust + LLVM + Git development environment on macOS.
- Create the compiler CLI and a test/CI skeleton.
- Document the architecture and build process.

**Implementation tasks**
- Verify Rust, Cargo, LLVM, Clang, Git, Python, CMake, Ninja, and host arch.
- Initialize the Git repository and make a clean first commit.
- Create a Cargo workspace (`navascript` CLI crate + module crates as needed).
- Implement `navascript --version`, `navascript --help`, `navascript build` (stub acceptable).
- Add a basic automated test (CLI smoke test) and a test runner script.

**Tests**
- `cargo test` passes.
- `navascript --version` and `navascript --help` print expected output.
- One integration test invokes the CLI.

**Acceptance criteria**
- A fresh clone builds and runs the CLI with documented commands.

**Known limitations (at Phase 0; resolved later)**
- `navascript build` did not compile anything yet — resolved in Phases 2–4.
- LLVM was not linked in Phase 0; full integration landed in Phase 4. This was
  a documented deviation from the spec's Phase 0 "Integrate LLVM" bullet — see
  ADR-0008 and ADR-0004.
- CMake/Ninja were not installed at the time and are not required by the chosen
  LLVM path; both were installed on 2026-11-21 for future C++/CMake work.
- At Phase 0 the CI workflow had not been exercised outside this machine; CI
  now installs LLVM on the macOS runner and runs fmt, clippy and the full suite.

---

## Phase 1 — Lexer and Parser

**Objectives**
- Tokenize and parse a useful subset of TypeScript syntax.
- Produce an AST with informative, renderable diagnostics.
- Establish the deep front-end seam defined in `docs/DESIGN.md` so later phases
  do not rewrite it.

**Implementation tasks** (follow the chosen deep-module interface)

1. `navascript-ast` — dependency-free leaf:
   - `FileId`, half-open byte-offset `Span`, `Program`, `Item`, statement and
     expression node types; debug printing for tests.
2. `navascript-lexer` — private internal seam of `navascript-parser`:
   - keywords, identifiers, numeric/string literals, operators, punctuation,
     line/block comments, EOF; byte spans on every token.
3. `navascript-parser` — external seam `parse(&str) -> ParseOutput`:
   - recursive-descent parsing with precedence, declarations (variable,
     function), statements (block, `if`, `return`, expression), expressions
     (literals, identifiers, binary/unary, calls, member).
   - panic-mode error recovery yielding a partial `Program`.
   - `Diagnostics`: deduplicated, sorted by span, 1-based line/column, caret
     rendering via `render(&str)`.
4. `navascript-driver` — wire `Command::Build` to call `parse`, render diagnostics and
   stop when `has_errors()`; no LLVM dependency.

**Tests**
- Token classes and spans via the public `tokenize` seam (`tests/lexer/`).
- Interface tests through `parse`: variables, functions, expressions, blocks,
  conditionals; assert results, not internal state.
- Syntax error tests asserting diagnostic message, line/column and rendered text.
- Driver integration test: a source file with a syntax error exits non-zero
  and prints readable diagnostics.

**Acceptance criteria**
- A basic `.ns` file parses into a valid AST, printed in debug form.
- A broken file produces sorted, positioned, renderable diagnostics.
- Front-end crates remain independent of LLVM (`cargo tree` shows no LLVM deps).

**Known limitations**
- No type checking yet — resolved in Phase 2.
- No classes, generics, interfaces, modules, loops — interfaces, type aliases
  and type checking landed in Phase 2; loops, classes, generics, unions, modules
  and closures were delivered in Phase 6 (see `PROGRESS.md`).
- JSX (`.nsx`) not supported.
- No `DiagnosticSink` trait yet: only one adapter exists (the returned value).
- Multi-file parsing is deferred; `Span` carries `FileId(0)` for now.

---

## Phase 2 — Type System

**Objectives**
- Implement a sound checker for the supported language subset.

**Implementation tasks**
- Symbol table + lexical scopes.
- Primitive types: `number`, `string`, `boolean`, `void`.
- Composite types: arrays, object literals, function signatures.
- Type aliases and interfaces.
- Type inference for variables and return expressions.
- Assignment compatibility and call-argument checking.

**Tests**
- Positive tests for inferred and annotated types.
- Negative tests: incompatible assignment, wrong arity, unknown identifier.
- Scope shadowing tests.

**Acceptance criteria**
- Correctly type-checks a meaningful subset of TypeScript.

**Known limitations**
- No overloads yet; generics, unions/intersections and structural typing were
  delivered in Phase 6 (see `PROGRESS.md`).
- `any`/`unknown` semantics deliberately restricted.

---

## Phase 3 — Intermediate Representation

**Objectives**
- Design NavaScript IR and lower the typed AST into it.

**Implementation tasks**
- IR data model: modules, functions, basic blocks, instructions, values, types.
- Control flow: branches, conditional branches, returns, phi nodes (or a
  documented alternative such as stack slots / SSA-with-block-arguments).
- Arithmetic, comparison, calls, loads/stores.
- IR verifier (type + dominance checks) and textual debug printer.

**Tests**
- AST → IR golden tests.
- Verifier rejects malformed IR (undefined values, bad terminators).

**Acceptance criteria**
- A valid TypeScript-like program lowers into verifiable IR.

**Known limitations**
- No optimization passes yet.
- Memory model still backend-agnostic.
- Target validation beyond the detected host (e.g. x86-64) is Phase 11.

---

## Phase 4 — Native Backend

**Objectives**
- Generate native ARM64 Mach-O executables via LLVM; no Node.js required.

**Implementation tasks**
- LLVM integration (crate or IR-text + `llc`/`clang` driver — see ADR-0004).
- IR → LLVM IR lowering for arithmetic, variables, functions, returns.
- Target detection from host, not hardcoded to ARM64.
- Object emission + Apple linker invocation; debug and release profiles.

**Tests**
- Compile and run small numeric programs end-to-end.
- The fibonacci acceptance program prints `55`.
- Cross-check output for debug vs release.

**Acceptance criteria**
- `navascript build` produces a native macOS executable that runs without Node.js.

**Known limitations**
- Strings, arrays, objects and runtime features were not supported in Phase 4;
  they landed in Phase 5 (ADR-0015 defers only closures and exceptions).
- x86-64 output not yet validated (targets exist in LLVM but path untested).
- Debug mode is `-O0`; source-level debug information is Phase 11.

---

## Phase 5 — Runtime and Standard Library

**Objectives**
- Support nontrivial programs with strings, collections, and errors.

**Implementation tasks**
- Memory allocation and a documented management strategy (see ADR-0005).
- String representation and operations.
- Arrays and objects.
- Fatal runtime error reporting for bounds/field errors (see ADR-0015).
- Minimal `console.log` and core stdlib surface.

Closures/function values and JavaScript-style exceptions are **Phase 6** work,
deferred by ADR-0015; they are not Phase 5 tasks.

**Tests**
- Runtime unit tests for allocation, strings, arrays, objects.
- End-to-end tests using collections and error paths.

**Acceptance criteria**
- Nontrivial applications run natively without Node.js.

**Known limitations**
- The arena allocator never reclaims memory during execution; a tracing
  collector or reference counting is a later upgrade (ADR-0005).
- Closures, function values and `throw`/`try` were deferred from Phase 5 to
  Phase 6 (ADR-0015). Closures and function values were delivered in Phase 6 by
  ADR-0027; `throw`/`try` remain pending. Runtime errors terminate the process
  with a diagnostic.

---

## Phase 6 — Language Expansion

**Objectives**
- Substantially broaden TypeScript compatibility.

**Implementation tasks (priority order)**
- Loops (`for`, `while`, `do-while`, `for-of`) — deferred from Phase 1, see the
  Phase 1 known limitations. **Delivered 2026-11-21** (status in `PROGRESS.md`).
- Arrow functions and the additional literals/operators listed in
  `docs/COMPATIBILITY.md` §6. **Delivered 2026-11-21** (status in `PROGRESS.md`).
- Optional calls (`a?.()`) and bigint literals (`123n`) — the remaining syntactic
  rows in `COMPATIBILITY.md` §6. **Delivered 2026-11-21** (status in
  `PROGRESS.md`).
- Generics — generic functions and interfaces with inferred/explicit type
  arguments, monomorphized lowering. **Delivered 2026-11-21** (status in
  `PROGRESS.md`; ADR-0023).
- Classes and inheritance — fields, constructors, methods, `this`/`super(...)`
  and inherited members, with static method dispatch. **Delivered 2026-11-21**
  (status in `PROGRESS.md`; ADR-0024).
- Interfaces and structural typing — union-aware assignability, optional
  properties, merged object layouts. **Delivered 2026-11-21** (ADR-0025).
- Union and intersection types — nullable reference unions with control-flow
  narrowing. **Delivered 2026-11-21** (ADR-0025).
- Modules, imports, exports, module resolution — import-graph resolution from
  the entry file, contextual `from`, validated exports. **Delivered 2026-11-24**
  (status in `PROGRESS.md`; ADR-0026).
- Closures and higher-order functions (with runtime from Phase 5).
  **Delivered 2026-11-24** (status in `PROGRESS.md`; ADR-0027).
- Exceptions (`throw`/`try`/`catch`/`finally`), deferred from Phase 5 (ADR-0015).
- Async/await and promises.

**Tests**
- Feature tests added per capability.
- Expanded compatibility suite.

**Acceptance criteria**
- A significantly larger TypeScript compatibility suite passes.

**Known limitations**
- Decorators, namespaces, JSX/TSX deferred unless evidence demands otherwise.
- Full `lib.d.ts` compatibility out of scope for this phase.
- Loops (including `break`/`continue`) landed first, then the additional
  literals/operators, arrow functions, optional member/index chaining, bigint,
  optional calls, generics, classes, modules and closures (see `PROGRESS.md`).
  Arrows require annotated parameters but may appear in any expression position;
  captures are shared cells. Optional chaining emits an `IsNull` branch but is
  limited to reference-typed results; a nullable receiver (`T | null`) is
  diagnosed by the checker, so `?.` cannot observe null yet (narrowing on
  `b != null` does work). Optional calls lower as direct calls for named
  functions and indirect calls through function values, with no null branch
  because there are no nullable callable values. Bigint is
  backed by a signed 64-bit integer: `+ - *` wrap on overflow while division,
  modulo and power are checked; arbitrary precision is out of scope. Generics
  cover functions and interfaces only: generic type aliases are out of scope,
  and there is no constraint syntax, generic inference across call boundaries
  beyond argument unification, or higher-kinded types. Classes use static
  method dispatch — there is no virtual dispatch, so a base-typed reference
  calls the base implementation. Class field initialisers are not supported;
  assign fields in a constructor.
  Unions must share one runtime representation: `number | string` and nullable
  primitives (`number | null`) are diagnosed, not silently miscompiled; only
  nullable reference types are usable today. Narrowing covers null tests in
  `if`, `while`, `&&`/`||` and branch bodies, not ternary or post-assignment
  flow. Modules share one namespace (no per-file scoping), resolve relative
  `.ns` paths (extensionless specifiers get `.ns`), and render diagnostics
  against the concatenated source rather than the originating file; `.d.ts`
  and package resolution are out of scope. Closures capture shared cells
  (ADR-0027), but a binding cannot be captured before it is initialised, so
  recursion must use a named function; parenthesised nested function types in
  annotations need a type alias. Iteration is limited to arrays/strings.

---

## Phase 7 — JavaScript / TypeScript Compatibility

**Objectives**
- Measure and document real compatibility against the official compiler.

**Implementation tasks**
- Test harness comparing behavior with `tsc` + Node where semantics overlap.
- Type checking, inference, runtime behavior, module resolution, stdlib, and
  JS interop test programs.
- Compatibility matrix and unsupported-feature list (see `COMPATIBILITY.md`).

**Tests**
- Reproducible corpus run with pass/fail/skip counts.
- Known-difference tests documenting intentional divergences.

**Acceptance criteria**
- A measured, reproducible compatibility report exists and is checked in.

**Known limitations**
- 100% compatibility is a long-term target, not a milestone gate.

---

## Phase 8 — Optimization

**Objectives**
- Improve generated code with evidence from benchmarks.

**Implementation tasks**
- Constant folding and dead code elimination (IR level).
- Function inlining.
- LLVM optimization pipelines (debug = minimal, release = aggressive).
- Escape analysis where practical; efficient memory layout.
- Reproducible benchmark suite.

**Tests**
- Optimizer unit tests (each pass preserves semantics).
- Benchmark suite with recorded baseline and results.

**Acceptance criteria**
- Measurable improvements on the defined benchmarks without correctness loss.

**Known limitations**
- Optimizations not sacrificing correctness for benchmark numbers.

---

## Phase 9 — Compiler Rewrite in NavaScript

**Objectives**
- Begin self-hosting by porting compiler components to NavaScript.

**Implementation tasks (port order)**
1. Lexer
2. Parser
3. AST structures
4. Symbol table
5. Semantic analyzer
6. Type checker
7. IR generation
8. Compiler driver

- The Rust compiler (`nova0`) stays functional throughout.

**Tests**
- Each ported component passes the same tests as its Rust counterpart.
- Differential tests: Rust vs NavaScript component output on identical inputs.

**Acceptance criteria**
- A substantial portion of the compiler is written in NavaScript.

**Known limitations**
- Rust driver still required to invoke the compiler until Phase 10.

---

## Phase 10 — Full Self-Hosting / Bootstrap

**Objectives**
- Achieve and verify the three-stage bootstrap (see `BOOTSTRAP.md`).

**Implementation tasks**
- Stage 0: Rust compiler compiles the NavaScript compiler source → `nova1`.
- Stage 1: `nova1` compiles the same source → `nova2`.
- Stage 2: `nova2` compiles the same source → `nova3` (identity check).
- Run the full compiler test suite at every stage.
- Compare program behavior across stages; pursue reproducible builds.

**Tests**
- All stages pass the same tests.
- Equivalent behavior on a representative program corpus.
- Byte-identical outputs where toolchain permits.

**Acceptance criteria**
- NavaScript compiles its own compiler from source with no hidden Rust dependency.

**Known limitations**
- Reproducible builds are "where practical," not an absolute gate.

---

## Phase 11 — Production Readiness

**Objectives**
- Make NavaScript usable by real developers.

**Implementation tasks**
- Package manager and dependency resolution.
- Standard library + documentation.
- Language server and editor support.
- Debugger integration (debug info).
- Cross-platform compilation (x86-64 macOS, then Linux/Windows).
- Security and reliability testing.
- Stable releases and versioning policy.

**Tests**
- End-to-end distribution tests on at least two targets.
- LSP smoke tests; debugger attach/breakpoint tests.

**Acceptance criteria**
- Developers can realistically build, debug, test, and distribute NavaScript apps.

**Known limitations**
- Timeline estimate: months 15–18+ from project start.

---

## Timeline estimate (planning, not commitments)

| Period | Goal |
| --- | --- |
| Weeks 1–2 | Foundation and lexer |
| Weeks 3–5 | Parser and AST |
| Weeks 6–9 | Type checker |
| Weeks 10–12 | IR and LLVM integration |
| Months 4–5 | Native executable generation |
| Months 6–7 | Runtime and language features |
| Months 8–10 | Compatibility and optimization |
| Months 11–14 | Begin self-hosting |
| Months 15–18+ | Bootstrap verification and production tooling |

First major achievement: compile `fibonacci(10)` to a native executable that
prints `55` with no Node.js involved.
