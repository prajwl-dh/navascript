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

- **Date:** 2026-11-20
- **Status:** Proposed (decide in Phase 0/4)
- **Context:** LLVM can be linked in-process via `llvm-sys` or driven as an
  external tool emitting textual IR.
- **Options:**
  - A: `llvm-sys` crate with `LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm`.
  - B: emit `.ll` and invoke `llc`/`clang` from the driver.
- **Decision:** Deferred. Phase 0 intentionally does not link LLVM, so no choice
  was needed there. Prototype both minimally at the start of Phase 4; choose
  based on build reliability on Homebrew macOS and debugging ergonomics.
- **Consequences:** Do not let this decision leak into the front end.

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

## Template for new decisions

```
## ADR-00NN — Title
- Date:
- Status: Proposed | Accepted | Superseded by ADR-00NN
- Context:
- Decision:
- Consequences:
```
