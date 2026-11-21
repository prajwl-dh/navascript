# Nava — Architecture Decision Records (ADRs)

Decisions are numbered and immutable once accepted. A superseding decision gets
a new number and references the old one. Keep entries short and factual.

---

## ADR-0001 — Project name: `nava`

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The source spec used `nova` in some places and `nava` in others.
- **Decision:** Use **nava** as the project/language name. The CLI binary is
  `nava`. Keep `nova0`/`nova1`/`nova2` as *bootstrap stage* names only, as
  defined by the bootstrap strategy, to avoid confusing the compiler name with
  stage artifacts.
- **Consequences:** All docs, paths, and commands use `nava`; stage artifacts
  use the `novaN` convention from `BOOTSTRAP.md`.

## ADR-0002 — Rust as the bootstrap implementation language

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Need a productive language for initial compiler development;
  self-hosting is a later goal.
- **Decision:** Implement `nova0` in Rust; progressively rewrite in Nava
  (Phase 9) while keeping Rust functional.
- **Consequences:** Rust toolchain is a build-time dependency only; final Nava
  compilation must not need it.

## ADR-0003 — LLVM as the backend

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Need native ARM64 codegen with optimization; hand-writing
  assembly would be impractical.
- **Decision:** Use LLVM 23.1.6 for instruction selection, register allocation,
  optimization, and object emission; emit LLVM IR from Nava IR.
- **Consequences:** LLVM is confined to the `codegen` module. Front end stays
  backend-independent. Do not hardcode target arch; query the triple.

## ADR-0004 — LLVM integration method

- **Date:** 2026-11-20 (decision 2026-11-21)
- **Status:** Accepted
- **Context:** LLVM can be linked in-process via `llvm-sys` or driven as an
  external tool emitting textual IR. The environment has Homebrew LLVM 23.1.6
  with AArch64 and X86 targets, Apple Clang 21, but no `rustup`, `cmake` or
  `ninja`. Phase 3 already produces a platform-independent IR and a textual
  printer.
- **Options:**
  - A: `llvm-sys` crate with `LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm`.
  - B: emit `.ll` and invoke `llc`/`clang` from the driver.
- **Decision:** **Option B.** The compiler emits textual LLVM IR, then invokes
  `llc` (from the detected LLVM prefix) to produce an object and `clang` to
  link it with a tiny C runtime. Reasons: no additional build tooling is
  required, the build stays reproducible on this machine, and the generated
  `.ll` is directly inspectable for debugging. The target triple is queried
  with `llvm-config --host-target`; it is never hardcoded to ARM64.
- **Consequences:** The `codegen` crate depends only on `nava-ir` and the
  standard library — no LLVM Rust bindings. Build reliability now depends on
  the external toolchain being on `PATH`/at the LLVM prefix; the driver reports
  a clear error if `llc`, `llvm-config` or `clang` is missing.

## ADR-0005 — Memory management strategy

- **Date:** 2026-11-20
- **Status:** Proposed (decide before Phase 5)
- **Context:** Need a documented strategy for allocation and reclamation; a
  tracing GC is a large undertaking early on.
- **Decision:** Deferred. Likely start with a simple, well-defined strategy
  (e.g. arena/bump allocation with explicit lifetimes or reference counting for
  strings/arrays) and record the exact choice, ownership rules, and known
  limitations here before implementing Phase 5.
- **Consequences:** Runtime ABI must be designed with the chosen strategy in
  mind; document divergence from JavaScript GC semantics.

## ADR-0006 — Numeric model

- **Date:** 2026-11-20
- **Status:** Proposed (decide in Phase 2)
- **Context:** TypeScript `number` is IEEE-754 double; native code may benefit
  from integer representations.
- **Decision:** Deferred. Default to 64-bit floating point for JavaScript-like
  semantics; consider integer specialization only with benchmark evidence and
  explicit correctness rules.
- **Consequences:** Avoid silent semantic changes when optimizing.

## ADR-0007 — Keep three goals independent

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Compatibility, native compilation, and self-hosting are often
  conflated.
- **Decision:** Track and report them separately; never claim one as evidence
  of another.
- **Consequences:** `COMPATIBILITY.md`, `PROGRESS.md`, and `BOOTSTRAP.md` each
  measure their own goal.

---

## ADR-0008 — Phase 0 scope: verify LLVM, defer linking to Phase 4

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The originating specification lists "Integrate LLVM" in the
  Phase 0 foundation tasks *and* in the Phase 4 native-backend tasks. Linking
  LLVM during Phase 0 would pull backend work forward and break the
  front-end/back-end separation. A review of Phase 0 flagged the mismatch
  between that bullet and the recorded "Phase 0 complete" status.
- **Decision:** Phase 0 verifies the LLVM toolchain (version, targets, tools,
  prefix) and records the integration plan (ADR-0004). Actual LLVM linking,
  code generation and linking of executables belong to Phase 4. Phase 0's
  "complete" status refers to the foundation scope defined in `ROADMAP.md`,
  with this deviation explicitly documented rather than silently assumed.
- **Consequences:** The Phase 0 exit condition is a building CLI and test
  suite, not a native executable. `PROGRESS.md` and `ROADMAP.md` state the
  deviation; no claim of LLVM integration is made before Phase 4.

---

## ADR-0009 — Shared diagnostics leaf crate

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Phase 2 needs the type checker to add diagnostics in the same
  shape as the parser. Keeping `Diagnostic`/`Diagnostics` inside `nava-parser`
  would force `nava-typechecker` to depend on the parser for a data type, even
  though it only consumes the AST.
- **Decision:** Move `Diagnostic`, `Severity`, `LineIndex` and `Diagnostics`
  into a new dependency-light crate `nava-diagnostics` with three real
  consumers (parser, type checker, driver). `nava-parser` re-exports them, so
  the Phase 1 `parse` seam is unchanged.
- **Consequences:** One sorting/dedup/rendering implementation for every
  front-end phase; no port or sink trait until a second real adapter exists.

---

## ADR-0010 — Semantic analysis lives inside the type checker

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The 12-module architecture lists both "Semantic analyzer" and
  "Type checker" as separate modules. In practice their Phase 2 work — scope
  building, duplicate declarations, name resolution, unknown identifiers — is
  inseparable from symbol-table construction and inference. Splitting them
  would create a shallow middle module that passes data straight through.
- **Decision:** Implement these semantic checks inside `nava-typechecker`
  behind its single `check` seam. `compiler/semantic/` remains reserved as a
  placeholder; if a genuinely independent semantic pass appears later, it can
  be extracted from the checker without changing the seam.
- **Consequences:** `nava-semantic` has no implementation in Phase 2. This is a
  documented deviation, not an omission: the checks are implemented and tested
  in `nava-typechecker`, satisfying "no feature claim without a passing test".

## ADR-0011 — Type-only `console` builtin

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The fibonacci acceptance program calls `console.log`, but the
  runtime stdlib is Phase 5. Without a type for `console`, Phase 2 rejects the
  very program the project uses as its first milestone.
- **Decision:** Seed the checker with a type-only `console` symbol exposing
  `log(value: unknown): void`. No runtime implementation is implied.
- **Consequences:** `examples/fibonacci.ts` type-checks in Phase 2. The real
  `console.log` implementation and its full signature belong to Phase 5.

---

## ADR-0012 — All tests live in the top-level tests/ tree

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Phase 0–2 placed unit tests in `#[cfg(test)]` modules inside each
  crate and binary tests in `compiler/driver/tests/`, while the documented
  repository structure reserved a top-level `tests/` tree that stayed empty.
  That mismatch made the test layout hard to discover and drifted from the
  spec's structure. Cargo only auto-discovers `tests/` at a package root.
- **Decision:** Keep **all** tests under `tests/`, organised by module
  (`ast/`, `diagnostics/`, `lexer/`, `parser/`, `symbol/`, `typechecker/`,
  `driver/`), with each file importing only public interfaces. Register each
  file as an explicit `[[test]]` target in `compiler/driver/Cargo.toml`; add
  dev-dependencies there. No inline test modules remain in `compiler/`.
- **Consequences:** One discoverable test location matching the documented
  repository structure. Tests can reach only public interfaces, so interfaces
  become the test surface by construction. Every new module must add its test
  file under `tests/` and a corresponding `[[test]]` target. `HELP` was made
  public so its content could be tested from outside the driver crate.

---

## ADR-0013 — Stack slots instead of SSA phi nodes in Nava IR

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** The Phase 3 roadmap allows phi nodes or a documented alternative
  such as stack slots. LLVM's `mem2reg` pass reliably promotes simple stack
  slots to SSA, and our front end already allocates one location per variable.
  Phi insertion during lowering would complicate the lowering, the verifier
  and the planned Nava rewrite for no measurable benefit at this stage.
- **Decision:** Nava IR uses explicit `alloca`/`store`/`load` stack slots for
  local variables. Control flow is expressed only with unconditional and
  conditional branches; there are no phi nodes. Instruction results are
  SSA-like values but are scoped to their block's dominance region.
- **Consequences:** The verifier checks dominance of value definitions and
  stack-slot allocations. Phase 4 lowers slots to LLVM `alloca` and lets
  optimization promote them. If profiling later shows a need, SSA can be
  introduced as a separate IR pass without changing the front-end seam.

## ADR-0014 — The IR lowering seam

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 3 needs a platform-independent representation that Phase 4
  can lower to LLVM. The driver already owns the parsed program, the typed
  program and the source text.
- **Decision:** `nava_ir::lower(&Program, &TypedProgram, &str) -> LowerOutput`
  is the single external seam. `LowerOutput` owns an `IrModule` plus
  `Diagnostics` in the same shape as earlier phases. `verify` and `print` are
  separate public functions because the verifier and IR debug output are
  explicit roadmap deliverables with real callers (driver, tests).
- **Consequences:** IR depends only on `nava-ast`, `nava-diagnostics` and
  `nava-symbol` — never LLVM. The driver calls parse → check → lower and
  renders diagnostics from all three uniformly.

---

## Template for new decisions

```
## ADR-00NN — Title
- Date:
- Status: Proposed | Accepted | Superseded by ADR-00NN
- Context:
- Decision:
- Consequences:
```
