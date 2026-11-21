# Nava — Progress Tracker

This is the **single source of truth** for project status. Update it after every
completed task. Anything marked `[~]` or `[!]` should have a note in the
[Work log](#work-log) explaining what is happening.

- **Status key:** `[ ]` not started · `[~]` in progress · `[x]` done · `[!]` blocked
- **Last updated:** 2026-11-21
- **Current phase:** Phase 4 complete — ready for Phase 5 (Runtime)
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
| Phase 5 — Runtime | [ ] Not started |
| Phase 6 — Language expansion | [ ] Not started |
| Phase 7 — Compatibility | [ ] Not started |
| Phase 8 — Optimization | [ ] Not started |
| Phase 9 — Self-hosting | [ ] Not started |
| Phase 10 — Bootstrap | [ ] Not started |
| Phase 11 — Production readiness | [ ] Not started |

**Overall:** 5 of 12 implementation phases complete.

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
  `nava-codegen`, assembled by `llc` and linked by `clang` (ADR-0004). The host
  triple comes from `llvm-config --host-target`; nothing is hardcoded to ARM64.
- [x] CMake 4.5.1 and Ninja 1.13.2 are installed via Homebrew as of
  2026-11-21. They are **not required** by the chosen textual-IR path, but are
  now available if the project later adopts `llvm-sys`, C++ tooling, or CMake-
  based builds.

---

## 3. Repository skeleton

```
nava/
├── compiler/          # Rust compiler modules (Phase 0+)
│   ├── lexer/ parser/ ast/ symbol/ semantic/ typechecker/
│   ├── ir/ optimizer/ codegen/ driver/
├── runtime/           # memory/ strings/ arrays/ objects/ exceptions/
├── bootstrap/         # stage0/ stage1/ stage2/
├── stdlib/
├── tests/             # lexer/ parser/ typechecker/ codegen/ runtime/ compatibility/
├── benchmarks/
├── docs/              # ROADMAP, ARCHITECTURE, LANGUAGE_SPEC,
│                      # COMPATIBILITY, BOOTSTRAP, DECISIONS, PROGRESS
├── examples/
├── scripts/
└── README.md
```

- [x] Directory structure created
- [x] `.gitkeep` placeholders added so empty dirs are tracked
- [x] `git init` completed on branch `main`
- [x] `Cargo.toml` workspace + 10 member crates (CLI driver + 9 module crates)
- [x] First commit — Phase 0

---

## 4. Milestone detail status

See `docs/ROADMAP.md` for objectives, tasks, tests, acceptance criteria and known
limitations for every milestone.

### Phase 0 — Foundation
- [x] Verify toolchain (done — see §2)
- [x] Create `Cargo.toml` workspace and `nava` CLI crate
- [x] Implement `nava --version` (also `-V`)
- [x] Implement `nava --help` (also `-h`, and no-argument default)
- [x] Implement `nava build` (stub: validates input, reports Phase 4 TODO)
- [x] Set up unit (3) + integration (13) test harness
- [x] CI workflow (`.github/workflows/ci.yml`) added
- [x] Document the build process (`docs/BUILD.md`)
- [x] Make the initial git commit
- **Exit condition met:** the workspace builds and the `nava` CLI runs its documented commands.
- **Verification:** `cargo build --workspace`, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and
  `cargo test --workspace` (16 passed, 0 failed) all succeed.

### Phase 1 — Lexer and Parser
- [x] Tokens: keywords, identifiers, numbers, strings, operators, comments
- [x] AST data structures (`FileId`, `Span`, `Program`, items, statements, expressions)
- [x] Variables, functions, expressions, blocks, `if`/`else`, top-level calls
- [x] Diagnostics with line + column, dedup/sort, caret rendering
- [x] Panic-mode recovery keeps later items after a syntax error
- [x] Driver wires `build` to `nava_parser::parse` and reports diagnostics
- **Exit condition met:** the fibonacci acceptance program parses; a broken
  file produces positioned diagnostics; no front-end crate depends on LLVM.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (40 passed, 0 failed) all succeed.
- **Deferred with a recorded deviation:** loops (`for`/`while`) were in the
  original Phase 1 spec; they are listed in `ROADMAP.md` Phase 1 limitations,
  tracked as unsupported in `COMPATIBILITY.md`, and scheduled for Phase 6.

### Phase 2 — Type system
- [x] Primitives: number, string, boolean, void, null
- [x] Arrays, objects, function signatures
- [x] Interfaces and type aliases
- [x] Inference and checking (assignment, arity, arguments, returns, identifiers)
- [x] Lexical scopes with shadowing and duplicate-declaration detection
- [x] Minimal built-in `console.log` surface for the fibonacci acceptance program
- **Exit condition met:** the fibonacci program type-checks; negative tests
  fail as expected; `nava build` reports type errors and exits non-zero.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (77 passed, 0 failed) succeed.

### Phase 3 — IR
- [x] Nava IR design (stack slots, SSA-like values; ADR-0013/0014)
- [x] AST → IR lowering (`nava_ir::lower`)
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
- [x] Minimal C runtime shim (`runtime/nava_runtime.c`) with a `main` bridge
- **Exit condition met:** `fibonacci(10)` prints `55` from a native Mach-O
  executable with no Node.js or JavaScript runtime.
- **Verification:** `cargo fmt --all -- --check`, clippy `-D warnings`, and
  `cargo test --workspace` (110 passed, 0 failed) succeed. The end-to-end tests
  run compiled binaries in both debug and release modes.

### Phase 5 — Runtime
- [ ] Memory allocation + chosen management strategy
- [ ] Strings, arrays, objects
- [ ] Closures
- [ ] Exceptions + runtime error reporting
- **Exit condition:** nontrivial apps run without Node.js.

### Phase 6 — Language expansion
- [ ] Generics, classes, inheritance
- [ ] Interfaces / structural typing
- [ ] Union / intersection types
- [ ] Modules, imports, exports
- [ ] Closures, higher-order functions
- [ ] Async/await, promises
- **Exit condition:** larger compatibility suite passes.

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

### Phase 9 — Self-hosting (rewrite in Nava)
- [ ] Lexer → Parser → AST → Symbol table → Semantic → Type checker → IR → Driver
- [ ] Rust compiler remains functional throughout
- **Exit condition:** substantial portion of compiler written in Nava.

### Phase 10 — Bootstrap
- [ ] Stage 0: Rust compiler compiles Nava compiler source
- [ ] Stage 1: `nova1` compiles the same source
- [ ] Stage 2: `nova2` compiles the same source
- [ ] Same test suite passes at every stage
- [ ] Equivalent program behavior across stages
- [ ] Reproducible builds where practical
- **Exit condition:** no hidden Rust dependency for normal Nava compilation.

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
- Created a Cargo workspace with 10 member crates: the `nava` CLI driver
  (`compiler/driver`, lib `nava_driver`) plus placeholder crates for `ast`,
  `lexer`, `parser`, `symbol`, `semantic`, `typechecker`, `ir`, `optimizer`, and
  `codegen`. Each placeholder records the phase in which it will be filled in.
- Implemented dependency-free CLI argument parsing in `cli.rs` with unit tests,
  and a thin `main.rs` that exits with the dispatcher's status code.
- `nava --version` / `-V` print `nava 0.1.0`; `--help` / `-h` and no arguments
  print usage; `build` validates arguments and input-file existence, then
  reports that native code generation is not implemented yet.
- Added 8 integration tests in `compiler/driver/tests/cli.rs` that invoke the
  real binary via `CARGO_BIN_EXE_nava`.
- Added `examples/fibonacci.ts` (the Phase 4 acceptance program), `scripts/build.sh`,
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
  `nava_parser::parse(&str) -> ParseOutput`; `nava-lexer` private behind it;
  `nava-ast` a public dependency-free leaf; no `DiagnosticSink` trait until a
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
- `nava-ast`: dependency-free leaf with `FileId`, half-open byte `Span`, `Program`,
  items, declarations, statements, expressions, operators and debug printing.
  Added `ItemKind::Expr` so script-style top-level calls (e.g. `console.log`)
  parse as valid programs.
- `nava-lexer`: private internal seam — byte-spanned tokens for identifiers,
  numbers, strings (with escapes), keywords, operators (longest match first),
  punctuation, both comment styles, EOF, plus lexer errors for unterminated
  strings/comments and unexpected characters.
- `nava-parser`: external seam `parse(&str) -> ParseOutput`. Recursive-descent
  parser with precedence, unary/postfix operators, calls, member access,
  `if`/`else` chains, panic-mode recovery, owned AST, and `Diagnostics` that
  are deduplicated, source-ordered and rendered with line/column and carets.
- `nava-driver`: `build` reads the source, calls `parse`, renders diagnostics,
  and exits non-zero on errors. Added `examples/bad_syntax.ts` for the error path.
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
- Added `nava-diagnostics` as a shared leaf: `Diagnostic`, `Severity`,
  `LineIndex` and `Diagnostics` now live in one crate used by the parser,
  checker and driver. The parser re-exports the types, so the Phase 1 seam is
  unchanged (ADR-0009).
- Extended `nava-ast` with `TypeExpr` (named, array, object, function), type
  aliases, interfaces, array/object literals and index expressions; the lexer
  recognises `=>`, `type` and `interface`, and the parser was extended for the
  new syntax.
- `nava-symbol` owns `Type` and `SymbolTable` (lexical scopes, shadowing,
  duplicate detection) as an internal seam.
- `nava-typechecker` exposes one deep seam,
  `check(&Program, &str) -> CheckOutput`, hiding two-pass declaration
  collection, scope handling, inference, alias/interface resolution and
  assignability. `TypedProgram` carries expression types keyed by span for
  Phase 3.
- Driver now runs parse → check and reports both diagnostic sets. Added
  `examples/type_error.ts` and a minimal built-in `console` type so the
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
  `nava_ir::lower` seam) before implementing.
- `nava-ir` now owns the complete IR data model (modules, functions, basic
  blocks, typed values, instructions, terminators, globals) behind four public
  functions: `lower`, `verify`, `print` and the data types themselves.
- Lowering handles functions, parameters, locals, arithmetic, comparisons,
  calls (including `console.log`), `if`/`else` control flow, string globals and
  a synthetic `main` for top-level statements. Allocations are emitted in the
  entry block so LLVM's `mem2reg` can promote them. Non-boolean `if` conditions
  get an explicit `ToBool` instruction, so the verifier's type rules stay strict.
- The verifier checks unique names/labels/ids, terminators, branch targets,
  dominance of values and slots, slot types, operand types and return types.
- Driver: `nava build` now runs parse → check → lower; `nava build --emit-ir`
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
- `nava-codegen` translates verified Nava IR to LLVM IR: typed functions,
  floating-point arithmetic, comparisons, branches, stack slots, calls and a
  synthetic `main`. It returns explicit errors for unsupported constructs
  (string globals, field/element access, opaque types).
- Driver pipeline is now parse → check → lower → generate → `llc` → `clang`
  link. Added `-o/--output`, `--release`, and kept `--emit-ir` for LLVM IR
  output. Added `runtime/nava_runtime.c` for the macOS `main` bridge and
  numeric `console.log`.
- **Major boundary crossed:** `nava build examples/fibonacci.ts -o fibonacci`
  produces a native executable that prints `55` with no Node.js.
- Added `tests/codegen/codegen.rs` and `tests/driver/native.rs` (8 new tests).
  Suite grew from 99 to 107 tests; fmt and clippy clean.

### 2026-11-21 — Phase 4 review fixes
- Two-axis review found 3 hard standards items, several spec defects and a few
  judgement smells. Applied before committing:
  - Fixed a per-function `ValueId` collision in codegen: the value-type map is
    now built per function, so multi-function programs with overlapping value
    IDs compile with correct call argument types.
  - `console.log` now has typed overloads: numbers call `nava_console_log_f64`,
    booleans call `nava_console_log_bool`. Previously `console.log(1 == 2)`
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
  binary tests that invoke `nava build` failed with `could not run llvm-config`.
- Added `nava_driver::build::native_tools_available()` as the single shared
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

---

## 6. Next actions (in order)

1. Start Phase 5 (Runtime): decide the memory-management strategy (ADR-0005),
   then add strings, arrays, objects and richer `console.log` support, replacing
   the minimal C shim.
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
