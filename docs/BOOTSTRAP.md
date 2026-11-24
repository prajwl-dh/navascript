# NavaScript — Bootstrapping & Self-Hosting

Self-hosting means NavaScript's compiler can compile its own source. It does **not**
by itself prove correctness or reproducibility; those require explicit
verification, described below.

Keep these goals independent:
- TypeScript compatibility (source language fidelity)
- Native compilation (machine code output)
- Self-hosting (compiler written in NavaScript)

## 1. Three-stage bootstrap

```
Stage 0  nova0 (Rust compiler)
         compiles the NavaScript compiler source  ──▶  nova1 (NavaScript compiler)

Stage 1  nova1 (NavaScript compiler)
         compiles the same source           ──▶  nova2 (NavaScript compiler)

Stage 2  nova2 (NavaScript compiler)
         compiles the same source           ──▶  nova3 (NavaScript compiler)
```

- **Stage 0 (`nova0`):** the original Rust bootstrap compiler. It is built once
  with Cargo and is the only artifact that depends on Rust.
- **Stage 1 (`nova1`):** the first compiler written in NavaScript, produced by
  `nova0`. At this point most/all compiler components are NavaScript source.
- **Stage 2 (`nova2`) / Stage 3 (`nova3`):** produced entirely by NavaScript
  compilers. Comparing behavior and outputs across these stages verifies the
  bootstrap.

Artifacts and sources will live under `bootstrap/stage0`, `bootstrap/stage1`,
and `bootstrap/stage2`. These directories do not exist yet — the empty
placeholders were removed in ADR-0016 and will be recreated in Phase 10.

## 2. Why not write the first compiler in NavaScript?

You generally shouldn't. Starting in Rust keeps early compiler work productive
and lets the language grow alongside the compiler. The rewrite happens
incrementally in Phase 9, while `nova0` remains functional.

## 3. Port order (Phase 9)

1. Lexer
2. Parser
3. AST structures
4. Symbol table
5. Semantic analyzer
6. Type checker
7. IR generation
8. Compiler driver

For each ported component:
- It passes the same tests as the Rust implementation.
- Differential tests compare its output with the Rust version on identical
  inputs.
- The Rust compiler stays runnable; never delete it during the rewrite.

## 4. Verification protocol (Phase 10)

For each stage:
- [ ] Build the compiler from the same source tree.
- [ ] Run the full compiler test suite; all tests must pass.
- [ ] Compile a representative program corpus and compare runtime behavior
      (stdout, exit codes) across stages.
- [ ] Compare generated compiler outputs where byte-equality is practical.
- [ ] Record toolchain versions and exact commands used.
- [ ] Confirm no hidden dependency on Rust for normal NavaScript compilation.

"Reproducible builds" are a goal **where practical** — toolchain nondeterminism
is documented rather than hidden.

## 5. Definition of success

- `nova3` exists and compiles NavaScript source without invoking `nova0`.
- All stages pass the same tests.
- Program behavior is equivalent across stages.
- Any remaining differences are documented and understood.

## 6. Risks

| Risk | Mitigation |
| --- | --- |
| Language lacks a feature needed by the compiler | Implement that feature in Rust compiler first; grow NavaScript to match |
| Semantic drift between Rust and NavaScript implementations | Differential tests at every port |
| Non-reproducible artifacts | Pin toolchain versions; document variance |
| Runtime not sufficient for compiler workload | Phase 5/8 must deliver adequate performance before Phase 9 |
