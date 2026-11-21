# Nava — Roadmap

This document defines the 12 milestones. Every milestone lists objectives,
implementation tasks, tests, acceptance criteria, and known limitations. Status
is tracked in `docs/PROGRESS.md`; this file is the stable definition of *what*.

**Guiding principles**
- Build one milestone at a time; each must produce something executable or
  independently testable.
- Correctness and modularity beat speed of code generation.
- Keep TypeScript compatibility, native compilation, and self-hosting as three
  *separate* goals. Progress on one must not be claimed as progress on another.

---

## Phase 0 — Project Foundation

**Objectives**
- Establish a working Rust + LLVM + Git development environment on macOS.
- Create the compiler CLI and a test/CI skeleton.
- Document the architecture and build process.

**Implementation tasks**
- Verify Rust, Cargo, LLVM, Clang, Git, Python, CMake, Ninja, and host arch.
- Initialize the Git repository and make a clean first commit.
- Create a Cargo workspace (`nava` CLI crate + module crates as needed).
- Implement `nava --version`, `nava --help`, `nava build` (stub acceptable).
- Add a basic automated test (CLI smoke test) and a test runner script.

**Tests**
- `cargo test` passes.
- `nava --version` and `nava --help` print expected output.
- One integration test invokes the CLI.

**Acceptance criteria**
- A fresh clone builds and runs the CLI with documented commands.

**Known limitations**
- `nava build` does not compile anything yet.
- LLVM is not linked yet.
- CMake/Ninja are not installed in the current environment (install needed).

---

## Phase 1 — Lexer and Parser

**Objectives**
- Tokenize and parse a useful subset of TypeScript syntax.
- Produce a typed AST with informative diagnostics.

**Implementation tasks**
- Token kinds: keywords, identifiers, numeric/string literals, operators,
  punctuation, comments (line + block), EOF.
- Source positions (line, column, byte offset) attached to tokens and AST nodes.
- AST: declarations (variable, function), statements (block, if, return,
  expression), expressions (literals, identifiers, binary/unary, calls, member).
- Recursive-descent parser with precedence handling.
- Error recovery and diagnostics with line/column.

**Tests**
- Token-level golden tests for each token class.
- Parser tests for variables, functions, expressions, blocks, conditionals.
- Syntax error tests asserting diagnostic text and position.

**Acceptance criteria**
- A basic `.ts` file parses into a valid AST, printed in debug form.

**Known limitations**
- No type checking yet.
- No classes, generics, interfaces, modules, loops (loops may land here if easy).
- `.tsx` not supported.

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
- No generics, unions/intersections, structural typing, or overloads yet.
- `any`/`unknown` semantics deliberately restricted.

---

## Phase 3 — Intermediate Representation

**Objectives**
- Design Nava IR and lower the typed AST into it.

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
- `nava build` produces a native macOS executable that runs without Node.js.

**Known limitations**
- Strings, arrays, objects and runtime features are not yet supported.
- x86-64 output not yet validated (targets exist in LLVM but path untested).

---

## Phase 5 — Runtime and Standard Library

**Objectives**
- Support nontrivial programs with strings, collections, and errors.

**Implementation tasks**
- Memory allocation and a documented management strategy (see ADR-0005).
- String representation and operations.
- Arrays and objects.
- Closures and function values.
- Exceptions / runtime error reporting.
- Minimal `console.log` and core stdlib surface.

**Tests**
- Runtime unit tests for allocation, strings, arrays, objects, closures.
- End-to-end tests using collections and error paths.

**Acceptance criteria**
- Nontrivial applications run natively without Node.js.

**Known limitations**
- GC strategy may start simple (e.g. arena/Bump allocator) and be upgraded.
- Exception semantics will initially differ from JavaScript.

---

## Phase 6 — Language Expansion

**Objectives**
- Substantially broaden TypeScript compatibility.

**Implementation tasks (priority order)**
- Generics.
- Classes and inheritance.
- Interfaces and structural typing.
- Union and intersection types.
- Modules, imports, exports, module resolution.
- Closures and higher-order functions (with runtime from Phase 5).
- Async/await and promises.

**Tests**
- Feature tests added per capability.
- Expanded compatibility suite.

**Acceptance criteria**
- A significantly larger TypeScript compatibility suite passes.

**Known limitations**
- Decorators, namespaces, JSX/TSX deferred unless evidence demands otherwise.
- Full `lib.d.ts` compatibility out of scope for this phase.

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

## Phase 9 — Compiler Rewrite in Nava

**Objectives**
- Begin self-hosting by porting compiler components to Nava.

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
- Differential tests: Rust vs Nava component output on identical inputs.

**Acceptance criteria**
- A substantial portion of the compiler is written in Nava.

**Known limitations**
- Rust driver still required to invoke the compiler until Phase 10.

---

## Phase 10 — Full Self-Hosting / Bootstrap

**Objectives**
- Achieve and verify the three-stage bootstrap (see `BOOTSTRAP.md`).

**Implementation tasks**
- Stage 0: Rust compiler compiles the Nava compiler source → `nova1`.
- Stage 1: `nova1` compiles the same source → `nova2`.
- Stage 2: `nova2` compiles the same source → `nova3` (identity check).
- Run the full compiler test suite at every stage.
- Compare program behavior across stages; pursue reproducible builds.

**Tests**
- All stages pass the same tests.
- Equivalent behavior on a representative program corpus.
- Byte-identical outputs where toolchain permits.

**Acceptance criteria**
- Nava compiles its own compiler from source with no hidden Rust dependency.

**Known limitations**
- Reproducible builds are "where practical," not an absolute gate.

---

## Phase 11 — Production Readiness

**Objectives**
- Make Nava usable by real developers.

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
- Developers can realistically build, debug, test, and distribute Nava apps.

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
