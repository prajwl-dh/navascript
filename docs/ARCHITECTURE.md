# Nava — Architecture

## 1. System overview

Nava is a source-to-native compiler with TypeScript-compatible syntax. The
compiler is written in Rust initially (`nova0`) and is progressively rewritten
in Nava itself. LLVM performs instruction selection, register allocation,
optimization, and native code generation; the compiler does not hand-write
ARM64 assembly.

```
TypeScript source (.ts)
        │
        ▼
  ┌───────────┐   ┌───────────┐
  │  Lexer    │──▶│  Parser   │   → AST + diagnostics
  └───────────┘   └───────────┘
                        │
                        ▼
             ┌────────────────────────┐
             │ Symbol table / Scopes  │
             │ Semantic analyzer      │
             │ Type checker           │
             └────────────────────────┘
                        │  typed AST
                        ▼
                 ┌─────────────┐
                 │  Nava IR    │   basic blocks, control flow
                 └─────────────┘
                        │
                        ▼
                 ┌─────────────┐
                 │ IR optimizer│   constant folding, DCE, inlining
                 └─────────────┘
                        │
                        ▼
          ┌──────────────────────────────┐
          │ LLVM backend (codegen)       │
          │ LLVM IR → object → link      │
          └──────────────────────────────┘
                        │
                        ▼
              Native Mach-O executable
              (ARM64 first, x86-64 later)
                        │
                        ▼
                   Native execution
```

**Critical boundary:** the parser, AST, semantic analysis, type checker, and IR
must remain independent of LLVM. Only the `codegen` module may depend on LLVM.

## 2. Module responsibilities

| Module | Location | Responsibility | Allowed deps |
| --- | --- | --- | --- |
| Lexer | `compiler/lexer/` | Source → token stream with positions | none |
| Parser | `compiler/parser/` | Tokens → AST; syntax diagnostics | lexer, ast |
| AST | `compiler/ast/` | Node definitions, spans, debug printing | none |
| Symbol table | `compiler/symbol/` | Scopes, symbols, name resolution | ast |
| Semantic | `compiler/semantic/` | Scope building, validity checks | ast, symbol |
| Type checker | `compiler/typechecker/` | Types, inference, assignability | ast, symbol, semantic |
| IR | `compiler/ir/` | IR data model, lowering, verifier | typechecker, ast |
| Optimizer | `compiler/optimizer/` | IR → IR transformation passes | ir |
| Codegen | `compiler/codegen/` | IR → LLVM IR / object; target query | ir, optimizer, LLVM |
| Driver | `compiler/driver/` | CLI, orchestration, link invocation | all above |
| Runtime | `runtime/` | Allocator, strings, arrays, objects, exceptions, closures | codegen ABI contract |
| Tests | `tests/` | Unit + integration + compatibility suites | — |

Each module owns its tests. A change to a module must be accompanied by its
tests passing (`cargo test -p <crate>` or equivalent).

## 3. Data flow and contracts

1. **Lexer → Parser:** token stream, each token carrying kind, lexeme, and span.
2. **Parser → AST:** immutable AST with spans for diagnostics; parsing never
   calls type checking or LLVM.
3. **AST → Typed AST:** symbol resolution and type annotation inserted without
   destroying source spans.
4. **Typed AST → IR:** lowering into functions/basic blocks/instructions. IR is
   platform-independent.
5. **IR → LLVM IR:** codegen maps IR types/instructions to LLVM constructs.
6. **LLVM → object → executable:** target-specific emission + system linker.

The runtime defines an ABI contract that codegen must honor (layout of strings,
arrays, objects, exception handling). This contract is versioned in
`runtime/README` when implemented and referenced from codegen tests.

## 4. LLVM strategy

LLVM 23.1.6 is installed at `/opt/homebrew/opt/llvm` with AArch64 and X86
targets built. Two integration options are under evaluation (see ADR-0004):

- **Option A — `llvm-sys` crate:** link libLLVM from Homebrew via
  `LLVM_SYS_231_PREFIX`. Tight integration, in-process passes, but version-
  sensitive build setup on macOS.
- **Option B — textual IR pipeline:** emit `.ll`, then call `llc`/`clang` from
  the driver. Simpler build, easier debugging, slower compile, more process
  plumbing.

Decision recorded in `docs/DECISIONS.md`. Do not hardcode ARM64: query the
target triple from LLVM and let `-target` drive emission.

## 5. Repository layout

```
compiler/     source modules + Cargo crates (workspace)
runtime/      allocator, strings, arrays, objects, exceptions
bootstrap/    stage0 (Rust), stage1 and stage2 (Nava compiler source + artifacts)
stdlib/       Nava standard library source
tests/        lexer, parser, typechecker, codegen, runtime, compatibility
benchmarks/   reproducible performance suite
docs/         this documentation set
examples/     sample .ts programs, starting with fibonacci.ts
scripts/      build/test/benchmark helper scripts
```

## 6. Build profiles

- **Debug:** minimal optimization, debug info, assertions and IR dumps enabled.
- **Release:** LLVM optimization pipeline enabled, debug info optional.

## 7. Testing strategy

- Unit tests next to each module; integration tests in `tests/`.
- Golden tests for lexer/parser/IR output.
- End-to-end tests compile `.ts` → executable → run → assert stdout/exit code.
- Compatibility corpus compares behavior against `tsc`/Node where applicable.
- Every stage of bootstrap runs the same test suite (see `BOOTSTRAP.md`).

## 8. Architectural invariants

1. LLVM is a backend detail, not a dependency of the front end.
2. No module claims a feature without a passing test.
3. IR is platform-independent; target specifics live in codegen.
4. The Rust compiler remains functional throughout self-hosting.
5. Diagnostics always carry source position information.
6. Unsupported TypeScript features are documented, never silently miscompiled.
