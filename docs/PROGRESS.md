# Nava — Progress Tracker

This is the **single source of truth** for project status. Update it after every
completed task. Anything marked `[~]` or `[!]` should have a note in the
[Work log](#work-log) explaining what is happening.

- **Status key:** `[ ]` not started · `[~]` in progress · `[x]` done · `[!]` blocked
- **Last updated:** 2026-11-20
- **Current phase:** Planning complete — Phase 0 not yet started
- **Tracking doc owner:** OpenCode agent (this project)

---

## 1. Current snapshot

| Item | Status |
| --- | --- |
| Project planning | [x] Done |
| Tracking documents created | [x] Done |
| Repository skeleton created | [x] Done |
| Git repository initialized | [x] Done (no commit yet) |
| Phase 0 — Foundation | [ ] Not started |
| Phase 1 — Lexer & Parser | [ ] Not started |
| Phase 2 — Type system | [ ] Not started |
| Phase 3 — IR | [ ] Not started |
| Phase 4 — Native backend | [ ] Not started |
| Phase 5 — Runtime | [ ] Not started |
| Phase 6 — Language expansion | [ ] Not started |
| Phase 7 — Compatibility | [ ] Not started |
| Phase 8 — Optimization | [ ] Not started |
| Phase 9 — Self-hosting | [ ] Not started |
| Phase 10 — Bootstrap | [ ] Not started |
| Phase 11 — Production readiness | [ ] Not started |

**Overall:** 0 of 12 implementation phases complete.

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
- [ ] Install CMake + Ninja (`brew install cmake ninja`) when Phase 0 needs them.
- [ ] Decide how the Rust compiler links LLVM: `llvm-sys` crate (needs `LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm`) vs. emitting textual LLVM IR and calling `llc`/`clang`. Decision deferred to Phase 0/4; see `docs/DECISIONS.md`.

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
- [x] `git init` completed (first commit deferred to Phase 0)
- [ ] `Cargo.toml` workspace + CLI crate — Phase 0
- [ ] First commit — Phase 0

---

## 4. Milestone detail status

See `docs/ROADMAP.md` for objectives, tasks, tests, acceptance criteria and known
limitations for every milestone.

### Phase 0 — Foundation
- [ ] Verify toolchain (done — see §2)
- [ ] Create `Cargo.toml` workspace and `nava` CLI crate
- [ ] Implement `nava --version`
- [ ] Implement `nava --help`
- [ ] Implement `nava build` (may be a stub initially)
- [ ] Set up unit + integration test harness
- [ ] Document the build process
- [ ] Make the initial git commit
- **Exit condition:** a minimal compiler project builds and runs its own CLI.

### Phase 1 — Lexer and Parser
- [ ] Tokens: keywords, identifiers, numbers, strings, operators, comments
- [ ] AST data structures
- [ ] Variables, functions, expressions, blocks, `if`, loops
- [ ] Diagnostics with line + column
- **Exit condition:** parse a basic `.ts` file into a valid AST.

### Phase 2 — Type system
- [ ] Primitives: number, string, boolean, void
- [ ] Arrays, objects, function signatures
- [ ] Interfaces and type aliases
- [ ] Inference and checking
- **Exit condition:** type-check a meaningful subset of TypeScript.

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

---

## 6. Next actions (in order)

1. Start Phase 0: create the Rust workspace + CLI skeleton (`nava --version`,
   `--help`, `build`), basic tests, and the first commit.
2. Install CMake + Ninja if Phase 0/4 requires them.
3. Resolve the LLVM-linking decision (see `docs/DECISIONS.md` ADR-0004).
4. Update this file after each completed task.

---

## 7. How to update this tracker

- Move a checkbox from `[ ]` to `[~]` when work starts, then `[x]` when its test
  passes and the behavior is verified. Use `[!]` with a note if blocked.
- Add a dated entry to the Work log for every meaningful change.
- Update the "Current phase" and "Last updated" fields at the top.
- Never mark something `[x]` unless a test proves it (AI Agent Development Rule 10).
