# NavaScript — Progress Tracker

This is the **single source of truth** for project status. Update it after every
completed task. Anything marked `[~]` or `[!]` should have a note in the
[Work log](#work-log) explaining what is happening.

- **Status key:** `[ ]` not started · `[~]` in progress · `[x]` done · `[!]` blocked
- **Last updated:** 2026-11-24
- **Current phase:** Phase 6 (Language expansion) in progress — loops (with
  `break`/`continue`), additional literals/operators, arrow functions, optional
  chaining, bigint literals, optional calls, generics, classes with inheritance,
  unions/intersections, full structural typing and modules complete
- **Tracking doc owner:** OpenCode agent (this project)

---

## 1. Current snapshot

| Item | Status |
| --- | --- |
| Project planning | [x] Done |
| Tracking documents created | [x] Done |
| Repository skeleton created | [x] Done |
| Git repository initialized | [x] Done (initial commit on `main`) |
| Phase 0 — Foundation | [x] Complete |
| Phase 1 — Lexer & Parser | [x] Complete |
| Phase 2 — Type system | [x] Complete |
| Phase 3 — IR | [x] Complete |
| Phase 4 — Native backend | [x] Complete |
| Phase 5 — Runtime | [x] Complete (closures/exceptions deferred, ADR-0015) |
| Phase 6 — Language expansion | [~] In progress (loops + jumps, operators, arrows, optional chaining, bigint, optional calls, generics, classes, unions, modules done) |
| Phase 7 — Compatibility | [ ] Not started |
| Phase 8 — Optimization | [ ] Not started |
| Phase 9 — Self-hosting | [ ] Not started |
| Phase 10 — Bootstrap | [ ] Not started |
| Phase 11 — Production readiness | [ ] Not started |

**Overall:** 6 of 12 implementation phases complete; Phase 6 is in progress
(loops + jumps, syntax expansion, arrows, optional chaining, bigint, optional
calls, generics, classes, unions/intersections, structural typing and modules
delivered; remaining expansion items pending).

---

## 2. Environment inventory (verified 2026-11-21)

| Tool | Required | Found | Status | Notes |
| --- | --- | --- | --- | --- |
| macOS | first-class target | 27.2.1 (26B131) | OK | Apple Silicon |
| CPU arch | detect, do not hardcode | `arm64` | OK | Backend must auto-detect |
| Rust / Cargo | bootstrap compiler | 1.100.0 (Homebrew) | OK | Installed via Homebrew, **no `rustup`** |
| rustfmt | formatting | 1.11.0 | OK | |
| clippy | linting | 0.1.100 | OK | |
| LLVM | optimization + codegen | 23.1.6 | OK | `/opt/homebrew/opt/llvm` |
| LLVM targets | AArch64 required, X86 later | AArch64 + X86 + many more | OK | Host target `arm64-apple-darwin` |
| LLVM tools | llc, opt, llvm-as, llvm-dis, llvm-link | all present | OK | In `/opt/homebrew/opt/llvm/bin` |
| Clang | native toolchain | Apple clang 21.0.0 | OK | Command Line Tools at `/Library/Developer/CommandLineTools` |
| Git | version control | 2.54.0 | OK | user.name / user.email configured |
| Python 3 | test + benchmark scripts | 3.14.6 | OK | via mise at `~/.local/share/mise/installs/python/3` |
| CMake | build infrastructure | 4.5.1 (Homebrew) | OK | Installed 2026-11-21; not required by the current backend |
| Ninja | build infrastructure | 1.13.2 (Homebrew) | OK | Installed 2026-11-21; not required by the current backend |

**Environment notes (resolved):**
- [x] LLVM integration method decided in Phase 4: textual `.ll` emitted by
  `navascript-codegen`, assembled by `llc` and linked by `clang` (ADR-0004). The host
  triple comes from `llvm-config --host-target`; nothing is hardcoded to ARM64.
- [x] CMake 4.5.1 and Ninja 1.13.2 are installed via Homebrew as of
  2026-11-21. They are **not required** by the chosen textual-IR path, but are
  now available if the project later adopts `llvm-sys`, C++ tooling, or CMake-
  based builds.

---

## 3. Repository skeleton

```
navascript/
├── compiler/          # Rust compiler crates (one public façade per crate)
│   ├── lexer/ parser/ ast/ symbol/ semantic/ typechecker/
│   ├── ir/ optimizer/ codegen/ driver/
├── runtime/           # navascript_runtime.h/.c + memory/ strings/ arrays/
│                      # objects/ console/ (one source per concern)
├── tests/             # ast, diagnostics, lexer, parser, symbol, typechecker,
│                      # ir, codegen, driver, runtime
├── docs/              # ROADMAP, ARCHITECTURE, LANGUAGE_SPEC, COMPATIBILITY,
│                      # BOOTSTRAP, DECISIONS, DESIGN, PROGRESS, BUILD
├── examples/
├── scripts/
└── README.md
```

- [x] Directory structure created (placeholder-only dirs removed in ADR-0016)
- [x] `.gitkeep` placeholders were used in Phase 0, then removed once the
  corresponding features had real files
- [x] `git init` completed on branch `main`
- [x] `Cargo.toml` workspace + 11 member crates (CLI driver + 10 module crates,
  including `navascript-diagnostics` added in Phase 2)
- [x] First commit — Phase 0

---

## 4. Milestone detail status

See `docs/ROADMAP.md` for objectives, tasks, tests, acceptance criteria and known
limitations for every milestone.

### Phase 0 — Foundation
- [x] Verify toolchain (done — see §2)
- [x] Create `Cargo.toml` workspace and `navascript` CLI crate
- [x] Implement `navascript --version` (also `-V`)
- [x] Implement `navascript --help` (also `-h`, and no-argument default)
- [x] Implement `navascript build` (stub: validates input, reports Phase 4 TODO)
- [x] Set up unit (3) + integration (13) test harness
- [x] CI workflow (`.github/workflows/ci.yml`) added
- [x] Document the build process (`docs/BUILD.md`)
- [x] Make the initial git commit
- **Exit condition met:** the workspace builds and the `navascript` CLI runs its documented commands.
- **Verification:** `cargo build --workspace`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (16 passed, 0 failed) all succeed.

### Phase 1 — Lexer and Parser
- [x] Tokens: keywords, identifiers, numbers, strings, operators, comments
- [x] AST data structures (`FileId`, `Span`, `Program`, items, statements, expressions)
- [x] Variables, functions, expressions, blocks, `if`/`else`, top-level calls
- [x] Diagnostics with line + column, dedup/sort, caret rendering
- [x] Panic-mode recovery keeps later items after a syntax error
- [x] Driver wires `build` to `navascript_parser::parse` and reports diagnostics
- **Exit condition met:** the fibonacci acceptance program parses; a broken
  file produces positioned diagnostics; no front-end crate depends on LLVM.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (40 passed, 0 failed) all succeed.
- **Deferred with a recorded deviation:** loops (`for`/`while`) were in the
  original Phase 1 spec; they stayed unsupported through Phase 5 and were
  delivered in Phase 6 (see the Phase 6 section below and `COMPATIBILITY.md`).

### Phase 2 — Type system
- [x] Primitives: number, string, boolean, void, null
- [x] Arrays, objects, function signatures
- [x] Interfaces and type aliases
- [x] Inference and checking (assignment, arity, arguments, returns, identifiers)
- [x] Lexical scopes with shadowing and duplicate-declaration detection
- [x] Minimal built-in `console.log` surface for the fibonacci acceptance program
- **Exit condition met:** the fibonacci program type-checks; negative tests
  fail as expected; `navascript build` reports type errors and exits non-zero.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (77 passed, 0 failed) succeed.

### Phase 3 — IR
- [x] NavaScript IR design (stack slots, SSA-like values; ADR-0013/0014)
- [x] AST → IR lowering (`navascript_ir::lower`)
- [x] Functions, variables, basic blocks, control flow (branches, condbr, returns)
- [x] IR validation (structural + dominance + type checks) + textual printing
- [x] Driver `build --emit-ir` exposes the debug output
- **Exit condition met:** the fibonacci fixture lowers to verified IR; all
  modules produced by the test corpus pass the verifier.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (93 passed, 0 failed) succeed.

### Phase 4 — Native backend
- [x] LLVM integration (textual IR + `llc` + `clang`, ADR-0004)
- [x] ARM64 codegen for arithmetic, vars, functions, returns
- [x] Mach-O executable generation + linking, with host target detection
- [x] Debug (`-O0`) and release (`--release`, `-O3`) modes
- [x] Minimal C runtime shim (`runtime/navascript_runtime.c`) with a `main` bridge
- **Exit condition met:** `fibonacci(10)` prints `55` from a native Mach-O
  executable with no Node.js or JavaScript runtime.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (110 passed, 0 failed) succeed. The end-to-end tests
  run compiled binaries in both debug and release modes.

### Phase 5 — Runtime
- [x] Memory allocation with a documented program-lifetime arena (ADR-0005)
- [x] Strings: literals, concatenation, `length`, indexing, equality, `console.log`
- [x] Arrays: literals, indexing, assignment, `length` (numbers, booleans, refs)
- [x] Objects: literals, field get/set with runtime field validation
- [x] Runtime error reporting (fatal bounds/field errors with clear messages)
- [ ] Closures / function values — deferred from Phase 5 to Phase 6, still pending (ADR-0015)
- [ ] Exceptions (`throw`/`try`) — deferred from Phase 5 to Phase 6, still pending (ADR-0015)
- **Exit condition met:** nontrivial programs using strings, collections and
  objects run natively without Node.js; runtime errors are reported.
- **Verification:** `cargo test --workspace` (120 passed, 0 failed), fmt and
  clippy clean; end-to-end tests exercise strings, arrays, objects, dynamic
  indices, string coercion and an out-of-bounds error path.

### Phase 6 — Language expansion
- [x] Loops (`while`, `do-while`, C-style `for`, `for-of` over arrays/strings,
  both `let x of` and `x of` forms), with `break`/`continue` — deferred from
  Phase 1; done 2026-11-21
- [x] Additional literals/operators: compound assignment (`+= -= *= /= %=`),
  `++`/`--`, `**`, ternary, hex/binary/octal literals, `_` separators,
  non-ASCII identifiers, `\x`/`\u` escapes — done 2026-11-21
- [x] Arrow functions as variable initialisers (annotated parameters, no
  captures) — done 2026-11-21
- [x] Optional chaining (`a?.b`, `a?.[i]`; reference-typed results) — done 2026-11-21
- [x] Optional call (`a?.()`) — parsed, type-checked and lowered as a direct
  call; null branch unobservable without nullable callables — done 2026-11-21
- [x] Bigint literals (`123n`, `0xffn`, `1_000n`) with a `bigint` type, integer
  arithmetic/comparisons, `++`/`--`, i64 collection slots and `console.log` —
  done 2026-11-21
- [x] Generics — generic functions and interfaces, inferred or explicit type
   arguments, monomorphized lowering — done 2026-11-21
- [x] Classes and inheritance — fields, constructors, methods, `this`,
   `super(...)`, inherited members/constructors, static dispatch — done 2026-11-21
- [x] Interfaces / structural typing (full) — union/intersection-aware
   assignability, optional properties and merged object layouts — done 2026-11-21
- [x] Union / intersection types with null narrowing — done 2026-11-21
- [x] Modules, imports, exports — import-graph resolution and export validation
  from the entry file; module-wide namespace — done 2026-11-24
- [ ] Closures, higher-order functions
- [ ] Async/await, promises
- [ ] Exceptions (`throw`/`try`/`catch`/`finally`) — deferred from Phase 5 (ADR-0015)
- **Current evidence:** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` (248 passed, 0 failed) succeed;
  `examples/loops.ns`, `examples/for_of.ns`, `examples/break_continue.ns`,
  `examples/operators.ns`, `examples/arrows.ns`, `examples/optional.ns`,
  `examples/bigint.ns`, `examples/optional_call.ns`, `examples/generics.ns`,
  `examples/classes.ns`, `examples/unions.ns` and `examples/modules.ns` run
  natively.
- **Exit condition:** larger compatibility suite passes (still pending — Phase 6
  is incomplete).

### Phase 7 — Compatibility
- [ ] Harness against official `tsc`
- [ ] Type checking / inference / runtime behavior tests
- [ ] Module resolution, stdlib, JS interop tests
- [ ] Measured compatibility report
- [ ] Unsupported-feature list
- **Exit condition:** reproducible compatibility report exists.

### Phase 8 — Optimization
- [ ] Constant folding, dead code elimination, inlining
- [ ] LLVM optimization passes
- [ ] Escape analysis where practical
- [ ] Reproducible benchmark suite + comparison
- **Exit condition:** measurable improvement on defined benchmarks.

### Phase 9 — Self-hosting (rewrite in NavaScript)
- [ ] Lexer → Parser → AST → Symbol table → Semantic → Type checker → IR → Driver
- [ ] Rust compiler remains functional throughout
- **Exit condition:** substantial portion of compiler written in NavaScript.

### Phase 10 — Bootstrap
- [ ] Stage 0: Rust compiler compiles NavaScript compiler source
- [ ] Stage 1: `nova1` compiles the same source
- [ ] Stage 2: `nova2` compiles the same source
- [ ] Same test suite passes at every stage
- [ ] Equivalent program behavior across stages
- [ ] Reproducible builds where practical
- **Exit condition:** no hidden Rust dependency for normal NavaScript compilation.

### Phase 11 — Production readiness
- [ ] Package manager, stdlib docs, language server, editor support
- [ ] Debugger integration, cross-platform, security/reliability testing
- [ ] Stable release + versioning policy
- **Exit condition:** developers can realistically build/debug/test/distribute.

---

## 5. Work log

### 2026-11-20 — Planning session
- Inspected environment; recorded versions and missing tools in §2.
- Created full directory skeleton and initialized Git.
- Authored all tracking documents: `ROADMAP.md`, `ARCHITECTURE.md`,
  `LANGUAGE_SPEC.md`, `COMPATIBILITY.md`, `BOOTSTRAP.md`, `DECISIONS.md`, and
  this `PROGRESS.md` (the central tracker).
- Wrote `README.md` with quick-start and links.
- **No compiler code written yet** — implementation begins with Phase 0 next.

### 2026-11-20 — Phase 0 (Foundation) completed
- Created a Cargo workspace with 10 member crates: the `navascript` CLI driver
  (`compiler/driver`, lib `navascript_driver`) plus placeholder crates for `ast`,
  `lexer`, `parser`, `symbol`, `semantic`, `typechecker`, `ir`, `optimizer`, and
  `codegen`. Each placeholder records the phase in which it will be filled in.
  (`navascript-diagnostics` was added in Phase 2, bringing the total to 11.)
- Implemented dependency-free CLI argument parsing in `cli.rs` with unit tests,
  and a thin `main.rs` that exits with the dispatcher's status code.
- `navascript --version` / `-V` print `navascript 0.1.0`; `--help` / `-h` and no arguments
  print usage; `build` validates arguments and input-file existence, then
  reports that native code generation is not implemented yet.
- Added 8 integration tests in `compiler/driver/tests/cli.rs` that invoke the
  real binary via `CARGO_BIN_EXE_navascript`.
- Added `examples/fibonacci.ns` (the Phase 4 acceptance program), `scripts/build.sh`,
  `scripts/test.sh`, `.gitignore`, and `docs/BUILD.md`.
- Fixed a clippy `trim_split_whitespace` warning and formatted with rustfmt.
- Verified: build OK, fmt check OK, clippy `-D warnings` OK, 20 tests passed.
  (Later refined to 16 tests — see the review-hardening entry below.)
- Deferred CMake/Ninja installation and the LLVM-link decision (ADR-0004) to
  Phase 4, since Phase 0 does not link LLVM.

### 2026-11-20 — Front-end module design
- Applied the codebase-design skill: framed the front-end seam as a deepening
  candidate (in-process pure computation), then compared three independently
  designed interfaces (minimal, flexible, caller-optimized).
- Chose a hybrid, recorded in `docs/DESIGN.md`: seam at
  `navascript_parser::parse(&str) -> ParseOutput`; `navascript-lexer` private behind it;
  `navascript-ast` a public dependency-free leaf; no `DiagnosticSink` trait until a
  second real adapter exists; multi-file support as a separate function when
  Phase 6 needs it.
- Linked the design from `docs/ARCHITECTURE.md`. No implementation code changed.

### 2026-11-20 — Code review applied, plan hardened
- Two-axis review of `7c8c7af...HEAD` found 0 hard standards violations, 7
  judgement-call items, and 2 spec gaps. Applied the useful parts:
  - CLI: deduplicated the no-argument flag handling (`no_arg_flag`), changed
    `Build { input: String }` to `PathBuf`, made `dispatch` private (no
    speculative public seam), and moved behaviour assertions from layered unit
    tests to the binary interface. Tests: 3 unit + 13 integration = 16, all
    passing; fmt/clippy still clean.
  - Docs: fixed ADR-0004's stale status and the `ParseOutput.diagnostics`
    field/method inconsistency in `docs/DESIGN.md`.
  - Plan: added a phase exit protocol and prerequisite/evidence table to
    `docs/ROADMAP.md`; rewrote Phase 1 tasks to follow the chosen deep-module
    seam; added ADR-0008 documenting the Phase 0/Phase 4 LLVM split so the
    "Phase 0 complete" status is honest.
  - Added `.github/workflows/ci.yml` (fmt, clippy, tests on macOS).
- Remaining review items deliberately not applied: per-module tests for empty
  placeholder crates (tests arrive with each phase's implementation), and
  actual LLVM linking (Phase 4 by ADR-0008).

### 2026-11-20 — Phase 1 (Lexer and Parser) implemented
- `navascript-ast`: dependency-free leaf with `FileId`, half-open byte `Span`, `Program`,
  items, declarations, statements, expressions, operators and debug printing.
  Added `ItemKind::Expr` so script-style top-level calls (e.g. `console.log`)
  parse as valid programs.
- `navascript-lexer`: private internal seam — byte-spanned tokens for identifiers,
  numbers, strings (with escapes), keywords, operators (longest match first),
  punctuation, both comment styles, EOF, plus lexer errors for unterminated
  strings/comments and unexpected characters.
- `navascript-parser`: external seam `parse(&str) -> ParseOutput`. Recursive-descent
  parser with precedence, unary/postfix operators, calls, member access,
  `if`/`else` chains, panic-mode recovery, owned AST, and `Diagnostics` that
  are deduplicated, source-ordered and rendered with line/column and carets.
- `navascript-driver`: `build` reads the source, calls `parse`, renders diagnostics,
  and exits non-zero on errors. Added `examples/bad_syntax.ns` for the error path.
- Tests: 7 lexer, 11 parser (through the `parse` seam), 1 AST, 3 driver unit,
  14 driver integration = 36, all passing; fmt and clippy clean. After the
  review fixes this became 40 (4 new parser tests).

### 2026-11-20 — Phase 1 review fixes
- Two-axis review of the uncommitted Phase 1 diff found 3 hard standards items,
  2 judgement calls, and several spec gaps. Applied before committing:
  - `parse` now has a 128-level depth guard, so deeply nested parentheses,
    unary chains and `if` chains produce a diagnostic instead of overflowing
    the stack (the DESIGN.md infallibility contract is now actually tested).
  - `if`/`else` accept a single unbraced statement as well as a block (wrapped
    in a synthetic block so the AST shape is unchanged).
  - Extracted the shared parse/recover/progress loop into `parse_until` and made
    diagnostic rendering reuse the single line-start table.
  - Removed the unused `Program::empty()` speculative API.
  - Added tests for dedup/tie-breaking, the debug form, deep nesting and
    unbraced branches; added the deferred syntax (arrows, hex/binary/bigint,
    compound operators, optional chaining, non-ASCII identifiers, `\u`/`\x`)
    to the unsupported-features register; scheduled loops in Phase 6.

### 2026-11-20 — Phase 2 (Type system) implemented
- Added `navascript-diagnostics` as a shared leaf: `Diagnostic`, `Severity`,
  `LineIndex` and `Diagnostics` now live in one crate used by the parser,
  checker and driver. The parser re-exports the types, so the Phase 1 seam is
  unchanged (ADR-0009).
- Extended `navascript-ast` with `TypeExpr` (named, array, object, function), type
  aliases, interfaces, array/object literals and index expressions; the lexer
  recognises `=>`, `type` and `interface`, and the parser was extended for the
  new syntax.
- `navascript-symbol` owns `Type` and `SymbolTable` (lexical scopes, shadowing,
  duplicate detection) as an internal seam.
- `navascript-typechecker` exposes one deep seam,
  `check(&Program, &str) -> CheckOutput`, hiding two-pass declaration
  collection, scope handling, inference, alias/interface resolution and
  assignability. `TypedProgram` carries expression types keyed by span for
  Phase 3.
- Driver now runs parse → check and reports both diagnostic sets. Added
  `examples/type_error.ns` and a minimal built-in `console` type so the
  fibonacci acceptance program type-checks.

### 2026-11-20 — Phase 2 review fixes
- Two-axis review found 5 hard standards items and several spec gaps. Applied
  before committing:
  - Return-type inference now runs inside `check_function`, after parameters
    and every top-level signature are registered. Fixed spurious
    "unknown identifier" errors for `function f(x: number) { return x; }` and
    for earlier functions calling later ones.
  - Local duplicate declarations are now reported (previously only top-level).
  - Alias chains resolve through multiple hops with cycle detection; cycles
    produce "circular type alias" instead of a misleading assignment error.
  - Property and element assignment targets (`p.x = ...`, `a[0] = ...`) are
    now type-checked, not silently skipped.
  - Comparisons of non-orderable types (e.g. `true < false`) are rejected;
    documented the remaining divergences (covariant function parameters,
    JavaScript-style string `+` coercion) in `COMPATIBILITY.md` §6.
  - Extracted the shared type-parameter parser loop; removed unused
    `SymbolKind::Type` and `SymbolTable::depth()`.
  - Documented two deliberate scope decisions: ADR-0010 (semantic analysis
    folded into the type checker) and ADR-0011 (type-only `console` builtin).
  - Fixed the stale `COMPATIBILITY.md` header and scope row, and added the
    missing `cargo fmt` exit evidence above.
  - Added six regression tests; the suite grew from 65 to 71 tests.
  - Populated the top-level `tests/` tree: `tests/parser/pipeline.rs` and
    `tests/typechecker/pipeline.rs` exercise the public `parse`/`check` seams
    from outside their crates, using the example fixtures. Registered as
    explicit Cargo test targets in `compiler/driver/Cargo.toml`. Suite is now
    77 tests. `docs/BUILD.md` documents the three test surfaces; module-local
    unit tests remain beside their implementations by design.

### 2026-11-20 — All tests consolidated under tests/
- Moved every `#[cfg(test)] mod tests` out of `compiler/*/src/` and the driver's
  binary tests out of `compiler/driver/tests/` into the top-level `tests/`
  tree: `tests/ast/`, `tests/diagnostics/`, `tests/lexer/`, `tests/parser/`,
  `tests/symbol/`, `tests/typechecker/`, `tests/driver/`.
- Each moved test now imports public items explicitly instead of `use super::*`.
  `HELP` was made public so the CLI help-content test can live outside the
  driver crate; no other interface needed widening.
- Registered every file as an explicit `[[test]]` target in
  `compiler/driver/Cargo.toml` and added crate dev-dependencies.
- Verified no inline test modules remain and no test files remain under
  `compiler/`. All 77 tests still pass; `docs/BUILD.md` documents the layout.
  Recorded as ADR-0012 and reflected in `ARCHITECTURE.md`, `DESIGN.md`,
  `ROADMAP.md`, `COMPATIBILITY.md` and `README.md`.

### 2026-11-21 — Phase 3 (Intermediate Representation) implemented
- Recorded ADR-0013 (stack slots instead of SSA phi) and ADR-0014 (the
  `navascript_ir::lower` seam) before implementing.
- `navascript-ir` now owns the complete IR data model (modules, functions, basic
  blocks, typed values, instructions, terminators, globals) behind four public
  functions: `lower`, `verify`, `print` and the data types themselves.
- Lowering handles functions, parameters, locals, arithmetic, comparisons,
  calls (including `console.log`), `if`/`else` control flow, string globals and
  a synthetic `main` for top-level statements. Allocations are emitted in the
  entry block so LLVM's `mem2reg` can promote them. Non-boolean `if` conditions
  get an explicit `ToBool` instruction, so the verifier's type rules stay strict.
- The verifier checks unique names/labels/ids, terminators, branch targets,
  dominance of values and slots, slot types, operand types and return types.
- Driver: `navascript build` now runs parse → check → lower; `navascript build --emit-ir`
  prints the textual IR. Added `tests/ir/{lower,verify,print}.rs` and one driver
  test for `--emit-ir`.
- Suite grew from 77 to 99 tests; fmt and clippy clean.

### 2026-11-21 — Phase 3 review fixes
- Two-axis review found 2 hard standards items, 3 judgement smells and several
  spec gaps. Applied before committing:
  - Fixed test-count claims (the docs said 99 before the suite reached 99; the
    suite is now genuinely 99 after adding 6 regression tests).
  - Verifier now rejects use-before-definition **within** a block (previously
    only cross-block dominance was checked), out-of-range parameters and
    references to unknown string globals.
  - `Instruction::Call` carries a `result_ty` so call results are typed and
    checked instead of always `Opaque`.
  - All stack-slot allocations now go in the entry block, matching ADR-0013's
    `mem2reg` rationale.
  - Unknown member calls are diagnosed rather than silently lowered as void;
    only known runtime builtins (currently `console.log`) are allowed.
  - Fixed a latent Phase 2 bug: inferred function return types were refreshed
    inside the parameter scope and then discarded. They are now written to the
    global scope, so `function p() {}` lowers to `-> void` with no phantom call
    result.
  - Functions that may fall through without returning now produce a diagnostic
    instead of a fabricated `0.0`.
  - Reverted an unrelated CI `macos-14 → macos-15` change to keep the diff
    focused.

### 2026-11-21 — Phase 4 (Native backend) implemented
- Resolved ADR-0004 in favour of textual LLVM IR driven by `llc`/`clang`:
  no extra build tooling, inspectable `.ll`, and the target triple is queried
  from `llvm-config --host-target` rather than hardcoded to ARM64.
- `navascript-codegen` translates verified NavaScript IR to LLVM IR: typed functions,
  floating-point arithmetic, comparisons, branches, stack slots, calls and a
  synthetic `main`. At the time it returned explicit errors for unsupported
  constructs (string globals, field/element access, opaque types); string and
  collection support landed in Phase 5, while opaque types are still rejected.
- Driver pipeline is now parse → check → lower → generate → `llc` → `clang`
  link. Added `-o/--output`, `--release`, and kept `--emit-ir` for LLVM IR
  output. Added `runtime/navascript_runtime.c` for the macOS `main` bridge and
  numeric `console.log`.
- **Major boundary crossed:** `navascript build examples/fibonacci.ns -o fibonacci`
  produces a native executable that prints `55` with no Node.js.
- Added `tests/codegen/codegen.rs` and `tests/driver/native.rs` (8 new tests).
  Suite grew from 99 to 107 tests; fmt and clippy clean.

### 2026-11-21 — Phase 4 review fixes
- Two-axis review found 3 hard standards items, several spec defects and a few
  judgement smells. Applied before committing:
  - Fixed a per-function `ValueId` collision in codegen: the value-type map is
    now built per function, so multi-function programs with overlapping value
    IDs compile with correct call argument types.
  - `console.log` now has typed overloads: numbers call `navascript_console_log_f64`,
    booleans call `navascript_console_log_bool`. Previously `console.log(1 == 2)`
    silently printed `1` through an ill-typed call.
  - Comparison and logical instructions are emitted with operand type checks
    (`icmp` for booleans, `fcmp` for numbers); unsupported operand types are
    reported as codegen errors instead of producing ill-typed LLVM IR.
  - Unsupported call argument types no longer silently widen to `double`.
  - Runtime number formatting no longer uses lossy `%g`; whole values, normal
    decimals and very small/large magnitudes now match TypeScript closely.
  - Build temp directories include an atomic counter to avoid PID collisions.
  - Fixed the stale "Last updated" tracker header.
- Added examples and tests for multi-function programs, boolean output and
  numeric formatting. Suite grew from 107 to 110 tests; fmt and clippy clean.
- Debug mode is `-O0` (unoptimized); source-level debug info remains Phase 11.
- Added `/fibonacci` to `.gitignore` for the CLI's default output name.

### 2026-11-21 — Documentation audit
- Reconciled every tracking document with the code as of Phase 4:
  - `PROGRESS.md`: environment notes now record the resolved LLVM integration
    and that CMake/Ninja are not required; stale next-action removed.
  - `COMPATIBILITY.md`: marked interfaces/type aliases, native arithmetic,
    comparisons and recursion as implemented; updated the header.
  - `LANGUAGE_SPEC.md`: replaced stale "planned" markers with the actual
    supported subset, corrected the keyword list to what the lexer recognizes,
    and recorded the acceptance program as achieved in Phase 4.
  - `DESIGN.md`: status updated from "Phase 1, not implemented" to implemented
    through Phase 4, including the backend seam.
  - `BUILD.md`: removed the obsolete llvm-sys discussion and Phase 0
    limitations; documented the resolved pipeline and current limitations.
  - `DECISIONS.md`: ADR-0006 (numeric model) marked accepted with its decision.
  - `README.md` and `ROADMAP.md`: removed stale CMake/Ninja and Phase 0/1 TODOs.
- No compiler code changed; the 110-test suite, fmt and clippy still pass.

### 2026-11-21 — CI fix: missing LLVM on GitHub runners
- GitHub's macOS runner does not ship LLVM at `/opt/homebrew/opt/llvm`, so two
  binary tests that invoke `navascript build` failed with `could not run llvm-config`.
- Added `navascript_driver::build::native_tools_available()` as the single shared
  tool-discovery seam; native tests and the two tool-dependent binary tests now
  skip cleanly when `llvm-config`, `llc` or `clang` is missing. `emit_native`
  returns an actionable error instead of a confusing OS error.
- CI now runs `brew install llvm` and exports `LLVM_SYS_231_PREFIX` from
  `brew --prefix llvm`, so the native path is actually exercised on the runner.
- Verified both modes locally: with a bogus prefix the suite passes with native
  tests skipped; with LLVM present all 110 tests execute and pass.

### 2026-11-21 — Toolchain update: CMake and Ninja installed
- Installed CMake 4.5.1 and Ninja 1.13.2 via Homebrew.
- Updated `PROGRESS.md` environment inventory (no longer marked MISSING),
  `README.md`, `BUILD.md` (prerequisites, verify commands, current limitations),
  `ROADMAP.md` and ADR-0004's historical context. They remain optional for the
  current textual-LLVM pipeline.
- No code changes; the 110-test suite, fmt and clippy remain green.

### 2026-11-21 — Phase 5 (Runtime) implemented
- Resolved ADR-0005 with a program-lifetime bump arena (16 MiB, 8-byte aligned,
  fatal error on exhaustion) and ADR-0015 deferring closures and exceptions.
- `runtime/navascript_runtime.c` grew from a numeric shim into a real runtime:
  strings (NUL-terminated arena buffers), arrays and objects with typed slot
  accessors, field-name tables, bounds checks and fatal error reporting.
- IR gained typed collection instructions (`ArrayNew/Get/Set/Length`,
  `ObjectNew/Get/Set`, `StringConcat/Equals/Length/CharAt`); `IrType` now models
  arrays and objects, and codegen emits the corresponding runtime calls.
- Fixed a latent type-map bug: expression types are now keyed by the full
  source span, because an identifier and its member expression share a start
  offset (`s.length`).
- Typechecker recognizes `string.length`, `array.length`, string indexing and
  string equality; lowering and codegen handle all of them natively.
- Added examples (`strings.ns`, `arrays.ns`, `objects.ns`, `runtime_error.ns`)
  and end-to-end tests. Suite grew from 110 to 117 tests; fmt and clippy clean.

### 2026-11-21 — Phase 5 review fixes
- Two-axis review found 5 hard standards items and several spec defects. All
  substantive findings were fixed before committing:
  - Object field order is now taken from the inferred type consistently for
    `ObjectNew`, `ObjectSet` and `ObjectGet` (previously source order mixed
    with the sorted field map, corrupting non-alphabetical literals).
  - Dynamic (non-constant) indices are converted `double → i64` in codegen;
    fractional indices are diagnosed instead of silently truncated.
  - Boolean array/object slots now use the correct 8-byte stride; a stray byte
    write had corrupted neighbouring numeric slots.
  - String coercion is implemented with a runtime `navascript_number_to_string`;
    `"a" + 1` and `1 == "1"` now produce well-typed IR and correct output.
  - Mixed coercion is no longer listed as unsupported; the stale "collection
    literals unsupported in IR" entry was removed.
  - Added C runtime unit tests (`tests/runtime/runtime_test.c`) covering arena
    alignment, strings, coercion, typed slots and cross-slot isolation, plus
    end-to-end tests for dynamic indices and non-alphabetical object fields.
  - Documented the runtime ABI in `runtime/README.md`, aligned the arena to
    8 bytes, and keyed expression types by `Span` instead of a raw offset pair.
- Suite grew from 117 to 120 tests; fmt and clippy clean.

### 2026-11-21 — Codebase restructuring (ADR-0016)
- Split the large implementation files along internal seams while keeping every
  compiler public façade and test target unchanged (only the runtime build
  plumbing `RUNTIME_C → RUNTIME_FILES` and `emit_native`'s parameter changed):
  - `ast`: `span.rs`, `expr.rs`, `types.rs`, `item.rs`.
  - `lexer`: `token.rs`, `scanner.rs`.
  - `parser`: `parser.rs`, `items.rs`, `types.rs`, `statements.rs`,
    `expressions.rs`.
  - `typechecker`: `checker.rs`, `declarations.rs`, `statements.rs`,
    `expressions.rs`, `types.rs`.
  - `ir`: `ir.rs`, `lower.rs`, `builder.rs`, `statements.rs`, `expressions.rs`,
    `verify.rs`, `checks.rs`, `print.rs`.
  - `codegen`: `lib.rs`, `runtime.rs`, `module.rs`, `instructions.rs`,
    `types.rs`.
- Split `runtime/navascript_runtime.c` into `runtime/navascript_runtime.h`,
  `runtime/navascript_runtime.c` (entry point only) and the per-concern sources
  `memory/memory.c`, `strings/strings.c`, `arrays/arrays.c`,
  `objects/objects.c`, `console/console.c`. The driver compiles all of them.
- Removed empty placeholder directories (`bootstrap/`, `stdlib/`,
  `benchmarks/`, `runtime/exceptions/`, `tests/compatibility/`) and all stale
  `.gitkeep` files; they will be recreated when their phases begin.
- Updated `ARCHITECTURE.md`, `DESIGN.md`, `BUILD.md`, `README.md` and
  `runtime/README.md`. All 120 tests, fmt and clippy still pass, and every
  example executable was rerun successfully.

### 2026-11-21 — Documentation accuracy audit
- Re-read every document and compared each claim with the code; fixed the
  discrepancies found before Phase 6 starts:
  - Phase 6 checklists now include loops, arrow functions/deferred literals and
    operators, and exceptions (ADR-0015); the Phase 5 task/test lists in
    `ROADMAP.md` no longer claim closures or exceptions, which are Phase 6 work.
  - `ARCHITECTURE.md` build profiles now describe the real toolchain
    (`llc -O0`/`-O3`, `clang -O0`/`-O2`; no debug info and no separate `opt`
    pipeline yet) instead of claiming debug info and assertions are enabled.
  - `LANGUAGE_SPEC.md` and `COMPATIBILITY.md` now reflect Phase 5 for strings,
    arrays, objects, member/index access and `console.log`; `null` is marked
    partial after verifying that an inferred `null` value fails codegen while
    `console.log(null)` prints `null`.
  - `BUILD.md`'s test-surface note, `BOOTSTRAP.md`'s future artifact paths and
    the `Program` snippet in `DESIGN.md` were corrected.
- Verified the environment versions and `cargo test --workspace` (120 passed).
  No compiler code changed.

### 2026-11-21 — Phase 6 (first increment): loops
- Added `while`, `do-while`, C-style `for` and `for-of` (arrays and strings)
  across every phase: lexer keywords, AST statement nodes, parser, type checker,
  IR lowering into basic-block back edges, and native codegen (no backend changes
  were needed). `for (let x of xs)` and `for (x of xs)` are both accepted; `of`
  is contextual, not a reserved keyword.
- Top-level loops are wrapped in `ItemKind::Stmt` and lowered into the synthetic
  `main`, so script-style examples work.
- `examples/loops.ns` and `examples/for_of.ns` compile and run natively. Added
  tests for tokenization, parsing, checking, IR back edges, LLVM IR and
  end-to-end output. Suite grew from 120 to 135 tests.
- Two-axis code review of `cad267b...HEAD` was applied: loop lowering was split
  into a private `ir/loops.rs` module (ADR-0016 file-size guidance), duplicated
  iterable-type dispatch was collapsed, and the review's spec findings were
  fixed as part of this change.
- **Not done in this increment:** `break`/`continue`, iteration over arbitrary
  iterables/objects, and the remaining Phase 6 items (arrow functions, generics,
  classes, unions, modules, closures, exceptions, async/await). These remain
  documented as unsupported.

### 2026-11-21 — Final documentation accuracy audit
- Re-read every document against the current tree after the Phase 6 loop commit
  (`6f937bb`) and corrected the remaining stale facts:
  - Workspace count is 11 member crates (`navascript-diagnostics` was added in Phase 2);
    the workspace layout lists in `README.md` and `BUILD.md` now include it.
  - Phase 1's deferred-loops note now says loops were delivered in Phase 6, and
    the Phase 6 roadmap task is annotated with its delivery date.
  - `ARCHITECTURE.md` no longer claims the runtime ABI covers exception handling;
    `DESIGN.md` status now reads "implemented through Phase 6 (loops)".
  - `LANGUAGE_SPEC.md` and `COMPATIBILITY.md` no longer promise object/array
    `console.log` support "until Phase 6"; the `break`/`continue` and general
    iterable gaps are in the unsupported-features register.
  - `BUILD.md` and `README.md` describe closures/exceptions as still pending
    Phase 6 work rather than deferred to a later phase.
- No compiler code changed; `cargo test --workspace` still passes 135 tests.

### 2026-11-21 — Phase 6 (second increment): literals and operators
- Added compound assignment (`+= -= *= /= %=`, including string `+=`),
  prefix/postfix `++`/`--`, right-associative `**` (LLVM `llvm.pow.f64`),
  ternary `?:`, hex/binary/octal literals with `_` separators, Unicode
  identifiers and `\xNN`/`\uNNNN`/`\u{...}` escapes.
- Shared radix-prefix knowledge between lexer and parser via `radix_prefix`;
  invalid `\x`/`\u` escapes now produce lex diagnostics instead of silently
  compiling to an empty string.
- Fixed two latent bugs found by the review: a `return` whose value contains a
  ternary attached the terminator to the stale block, and ternary expressions
  in variable/expression statements left the final block unterminated.
- Split the three files that exceeded ADR-0016's size guidance into private
  modules: `ir/operators.rs`, `lexer/scanner/{number,string}.rs` and
  `parser/operators.rs`. All public seams are unchanged.
- Added `examples/operators.ns` and tests across lexer/parser/typechecker/IR/
  codegen/native surfaces. Suite grew from 135 to 149 tests; fmt and clippy
  clean.
- **Still pending:** arrow functions, optional chaining, bigint, closures,
  exceptions, generics, classes, unions, modules and async/await.

### 2026-11-21 — Phase 6 (third increment): arrow functions
- Added arrow expressions (`x => e`, `(a, b) => e`, optional return annotation,
  expression or block bodies) to the AST, parser and type checker. Arrow
  initialisers lower to named IR functions; calls use the binding name directly.
- Parameters require type annotations: unannotated parameters were otherwise
  inferred as `Unknown` and could silently compile as `number`. Captured locals
  and arrow values used outside calls are diagnosed in IR rather than
  miscompiled.
- Added private `parser/arrows.rs` and `typechecker/arrows.rs` so the touched
  implementation files stay within ADR-0016's size guidance; moved arrow
  function construction onto `FunctionBuilder` to keep builder state local.
- Two-axis code review of `baa6619...HEAD` was applied: the overlong parser file
  was shortened, the Function-to-IR mapping was scoped to arrow return types,
  and the soundness and documentation findings were fixed.
- Added `examples/arrows.ns` and tests across parser/typechecker/IR/native
  surfaces. Suite grew from 149 to 156 tests; fmt and clippy clean.
- **Still pending:** optional chaining, bigint, function values/closures,
  exceptions, generics, classes, unions, modules and async/await.

### 2026-11-21 — Phase 6 (fourth increment): optional chaining
- Added `?.` as a longest-match lexer token, `a?.b` and `a?.[i]` parsing,
  type-checking (reference-typed results only; scalar results diagnosed), and
  IR lowering with a dedicated `IsNull` instruction, null/value blocks and a
  result stack slot. Codegen emits `icmp eq ptr ..., null` for `IsNull`.
- Rejected assignment through optional chaining in both the checker and IR.
  At this point `a?.()` produced a parse diagnostic (it was implemented in the
  final Phase 6 increment below), never a silent miscompile.
- Added private `parser/optional.rs` and `typechecker/optional.rs`, plus
  `ir/calls.rs`, keeping touched implementation files within ADR-0016.
- Two-axis review of `5c5695c...HEAD` was applied: the global `==` change was
  replaced by the explicit `IsNull` instruction, the optional-access data is
  now an enum-backed struct (no invalid states), and the double `target.kind`
  switch was folded into one match.
- Added `examples/optional.ns` and lexer/parser/typechecker/IR/native tests.
  Suite grew from 156 to 163 tests; fmt and clippy clean.
- **Known limitation:** the null branch is unobservable until nullable values
  exist; results are typed `T` until union types land. Optional calls and bigint
  were still pending at this point (both delivered in the final increment).

### 2026-11-21 — Phase 6 (fifth increment): break and continue
- Added `break`/`continue` keywords, AST statements, parser support, a
  `loop_depth` check in the type checker (diagnosing use outside loops), and IR
  lowering through a `LoopContext` stack (`break` → exit, `continue` → header,
  condition block or `for` update block).
- Loop-context setup/teardown is shared by `FunctionBuilder::with_loop`; the
  `for-of` lowering moved into a private `ir/for_of.rs` module to stay within
  ADR-0016.
- Two-axis review of `9f1e9cc...HEAD` was applied: a `has_jump` flag now keeps
  target blocks alive for both `break` and `continue` (the original `has_break`
  let `do { continue; } while (...)` and `for (;;) { continue; }` branch to a
  deleted block); the duplicated push/lower/pop and `do.cond` emission were
  factored into helpers.
- Extended `examples/break_continue.ns` and tests for do-while, `for (;;)`,
  `for-of`, nested loops and trailing `continue`. Suite grew from 163 to 169
  tests; fmt and clippy clean.

### 2026-11-21 — Full documentation accuracy audit (Phase 6 state)
- Re-read every doc against the tree at commit `740839b` and corrected the
  remaining drift: `README.md` claimed `break`/`continue` and arrow functions
  were unimplemented; the tracker header/snapshot and next-actions omitted
  optional chaining and `break`/`continue`; `DESIGN.md` and the
  `COMPATIBILITY.md` header listed only the first two Phase 6 increments;
  `BUILD.md`'s native-test row omitted arrows, optional chaining and loop
  jumps.
- Verified the current facts directly: 169 tests pass, 11 workspace members,
  18 `.ns` examples, and the IR's private modules (`calls`, `for_of`, `loops`,
  `operators`). No compiler code changed.

### 2026-11-21 — Phase 6 (final increment): bigint and optional calls
- **Bigint literals:** `TokenKind::BigInt` recognises `123n`, `0xffn`, `1_000n`
  and other radix/separator forms; `decode_bigint` decodes via `i128` and
  diagnoses values outside the signed 64-bit range, and a `n` suffix on a
  fractional/exponent form is a lex diagnostic. `123nfoo` is *not* treated as a
  bigint (the suffix must be word-final).
- Added a `bigint` primitive through the checker (`Type::BigInt`), IR
  (`IrType::BigInt`, `Value::BigInt(i64)`, `Instruction::BigIntBinary`,
  `UnaryOp::NegBigInt`) and codegen (LLVM `i64`). Bigint supports `+ - * / % **`,
  comparisons/equality, unary `-`, `++`/`--`, bigint arrays/objects and
  `console.log`. Division, modulo and power use checked runtime helpers
  (`runtime/bigint/bigint.c`); `+ - *` follow LLVM i64 wrap semantics.
- **Optional calls:** `?.()` is parsed as a `Call` with an `optional` flag,
  type-checked for callability/arity like a direct call, and lowered as a direct
  call. No null branch is emitted: NavaScript has no first-class callable values, so
  there is nothing nullable to test (documented in ADR-0020).
- Unified IR instruction result-type mapping in private `ir/results.rs`, shared
  by the verifier and codegen (it also fixed a pre-existing `And`/`Or`
  divergence). Split `typechecker/bigint.rs`, `ir/values.rs` and `ir/results.rs`
  to stay within ADR-0016's size guidance; extracted the shared call-tail parser
  into `parser/calls.rs` and the separator stripper into `parser/parser.rs`.
- Two-axis code review of `c75e46f...HEAD` (working tree) was applied: unified
  result types, removed unreachable bigint `And`/`Or` result handling, corrected
  the runtime's overflow comment, added verifier negative tests, and kept files
  within ADR-0016. The Spec axis' scope question was resolved by noting that the
  §6 row's own reason requires a checker/runtime type, making arithmetic the
  minimum that makes literals testable.
- Added `examples/bigint.ns`, `examples/optional_call.ns` and tests across
  lexer/parser/typechecker/IR/verifier/codegen/native/runtime surfaces. Suite
  grew from 169 to 192 tests; fmt and clippy clean.

### 2026-11-21 — Phase 6 (generics increment)
- Added generic functions (`function id<T>(x: T): T`) and generic interfaces
  (`interface Box<T> { value: T; }`). Type arguments are inferred by unifying
  declared parameter types with actual argument types, or supplied explicitly
  (`id<number>(1)`). Type-argument arity and mismatches are diagnosed, and
  `Box<number>` resolves to a concrete object type.
- Monomorphization: IR emits one specialized function per distinct type-argument
  combination (`navascript_id__number`, `navascript_id__string`), keyed by a structural
  mangling that distinguishes object shapes and disambiguates user-defined
  names. Recursive generic calls reuse the reserved specialization.
- Divergent-resolver review fixes: function signatures now come from the
  checker's resolved types rather than AST annotations, bad generic references
  emit diagnostics (no silent `Type::Error`), angle-list parsing is shared, and
  `specialize_if_generic` / mangling live in a private `ir/generics.rs`.
  Generic type aliases were deliberately left out of scope.
- Two-axis code review of `33c5656...HEAD` (working tree) was applied: fixed the
  object-type mangling collision, the silent generic-resolution errors, the
  feature-envy lowering helper, and the shared angle-argument parser.
- Added `examples/generics.ns` and tests across parser/typechecker/IR/codegen/
  native surfaces. Suite grew from 192 to 213 tests; fmt and clippy clean.

### 2026-11-21 — Phase 6 (classes and inheritance increment)
- Added class declarations with fields, constructors, methods, `this`,
  `super(...)`, `extends`, and `new`. Methods lower to named IR functions with
  `this` as their first parameter; dispatch is static (ADR-0024).
- Class instances carry a nominal `Type::ClassInstance` with base-first field
  order. This fixed two soundness bugs found by review: nondeterministic
  dispatch from structural class identity, and inherited field indices shifting
  when a derived field sorts before a base field. `examples/classes.ns` now
  covers both (`Shifted.sum()` returns 30, not a reinterpreted pointer).
- Constructors are inherited when a derived class declares none, and
  `super(args)` calls the base constructor on `this`.
- Two-axis code review of `6e48dd8...HEAD` (working tree) was applied: removed
  the structural dispatch fallback, preserved base-first field layout, added
  constructor inheritance and `super(...)`, split oversized class modules, and
  corrected the native expectation from the mis-indexed `4` to `9`.
- Added `examples/classes.ns` and tests across lexer/parser/typechecker/IR/
  codegen/native surfaces. Suite grew from 213 to 227 tests; fmt and clippy
  clean.

### 2026-11-21 — Phase 6 (type-system block): unions and structural typing
- Added `|` / `&` type syntax with union/intersection `Type` variants,
  normalization, order-independent assignability, and merged intersection
  layouts. Optional properties (`field?: T`) resolve to `T | null` and omitted
  reference fields are initialized to null (ADR-0025).
- Null narrowing in `if`, `while`, logical conditions and branch bodies, backed
  by the existing `IsNull` instruction — the null branch of `a?.b` is now
  observable for nullable reference types.
- Two-axis code review of `b251e15...HEAD` (working tree) was applied: rejected
  unions with mixed runtime representations and nullable primitives (they
  previously produced invalid IR), fixed order-sensitive union assignability,
  reverted an out-of-scope global pointer-equality change, extended narrowing to
  loops and `&&`/`||`, and removed duplicated object-field resolution.
- Added `examples/unions.ns` and tests across lexer/parser/typechecker/IR/
  codegen/native surfaces. Suite grew from 227 to 243 tests; fmt and clippy
  clean.

### 2026-11-24 — Phase 6 (modules increment)
- Added `import { ... } from "..."` and `export <declaration>` syntax. `from`
  stays contextual, so `const from = 1` still lexes (ADR-0026).
- `navascript_driver::modules::load` walks the import graph from the entry file,
  deduplicates visited paths (handling cycles and diamonds), resolves
  extensionless specifiers to `.ns`, and concatenates sources in dependency
  order. The compiler's parse/check/lower/generate seams still see one program.
- Exported declarations are collected from `export` wrappers and imports are
  validated against them; classes, interfaces and aliases are checked too, not
  just symbol-table entries.
- Two-axis code review of `8ce8397...HEAD` (working tree) was applied: made
  `from` contextual, added export validation and extensionless resolution,
  fixed exported alias cycle checking, cached sources during graph discovery,
  and added module tests (lexer/parser/checker/IR/native plus a transitive
  extensionless build).
- Known limitations recorded in ADR-0026: one module-wide namespace, no
  `.d.ts`/package resolution, and diagnostics rendered against the combined
  source rather than the originating file.
- Added `examples/modules.ns` and `examples/math.ns`. Suite grew from 243 to
  248 tests; fmt and clippy clean.

---

## 6. Next actions (in order)

1. Continue Phase 6 (Language expansion) in the priority order defined in
   `ROADMAP.md`: closures and exceptions (ADR-0015), async/await. Loops
   (including `break`/`continue`), the additional literals/operators, arrow
   functions, optional chaining, bigint, optional calls, generics, classes,
   unions/intersections, structural typing and modules are done.
2. Add x86-64 target validation once LLVM targets beyond the host are needed.
3. Update this file after each completed task, using the phase exit protocol in
   `docs/ROADMAP.md`.

---

## 7. How to update this tracker

- Move a checkbox from `[ ]` to `[~]` when work starts, then `[x]` when its test
  passes and the behavior is verified. Use `[!]` with a note if blocked.
- Add a dated entry to the Work log for every meaningful change.
- Update the "Current phase" and "Last updated" fields at the top.
- Never mark something `[x]` unless a test proves it (AI Agent Development Rule 10).
