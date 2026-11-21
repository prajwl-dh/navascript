# Nava — Progress Tracker

This is the **single source of truth** for project status. Update it after every
completed task. Anything marked `[~]` or `[!]` should have a note in the
[Work log](#work-log) explaining what is happening.

- **Status key:** `[ ]` not started · `[~]` in progress · `[x]` done · `[!]` blocked
- **Last updated:** 2026-11-20
- **Current phase:** Phase 2 complete — ready for Phase 3 (Intermediate Representation)
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
| Phase 3 — IR | [ ] Not started |
| Phase 4 — Native backend | [ ] Not started |
| Phase 5 — Runtime | [ ] Not started |
| Phase 6 — Language expansion | [ ] Not started |
| Phase 7 — Compatibility | [ ] Not started |
| Phase 8 — Optimization | [ ] Not started |
| Phase 9 — Self-hosting | [ ] Not started |
| Phase 10 — Bootstrap | [ ] Not started |
| Phase 11 — Production readiness | [ ] Not started |

**Overall:** 3 of 12 implementation phases complete.

---

## 2. Environment inventory (verified 2026-11-20)

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
| CMake | build infrastructure | — | [!] MISSING | Install before heavy LLVM/C++ interop |
| Ninja | build infrastructure | — | [!] MISSING | Install alongside CMake |

**Environment blockers:**
- [ ] Install CMake + Ninja (`brew install cmake ninja`). Deferred in Phase 0 — not needed until deeper LLVM/C++ work in Phase 4+.
- [ ] Decide how the Rust compiler links LLVM: `llvm-sys` crate (needs `LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm`) vs. emitting textual LLVM IR and calling `llc`/`clang`. Deferred to Phase 4; Phase 0 intentionally does not link LLVM. See `docs/DECISIONS.md` ADR-0004.

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
- [ ] Nava IR design
- [ ] AST → IR lowering
- [ ] Functions, variables, basic blocks, control flow
- [ ] IR validation + debug printing
- **Exit condition:** valid IR produced from a TypeScript-like program.

### Phase 4 — Native backend
- [ ] LLVM integration
- [ ] ARM64 codegen for arithmetic, vars, functions, returns
- [ ] Mach-O executable generation + linking
- [ ] Debug and release modes
- **Exit condition:** fibonacci(10) prints `55` natively, no Node.js.

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

---

## 6. Next actions (in order)

1. Review the Phase 2 diff and apply improvements before committing it.
2. Start Phase 3 (IR) in `nava-ir`: functions, basic blocks, control flow,
   arithmetic and calls, plus a verifier and textual debug output.
3. Install CMake + Ninja when Phase 4 needs them.
4. Resolve the LLVM-linking decision (see `docs/DECISIONS.md` ADR-0004) at the
   start of Phase 4.
5. Update this file after each completed task, using the phase exit protocol in
   `docs/ROADMAP.md`.

---

## 7. How to update this tracker

- Move a checkbox from `[ ]` to `[~]` when work starts, then `[x]` when its test
  passes and the behavior is verified. Use `[!]` with a note if blocked.
- Add a dated entry to the Work log for every meaningful change.
- Update the "Current phase" and "Last updated" fields at the top.
- Never mark something `[x]` unless a test proves it (AI Agent Development Rule 10).
