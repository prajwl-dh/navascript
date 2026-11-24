# NavaScript — Architecture

## 1. System overview

NavaScript is a source-to-native compiler with TypeScript-compatible syntax. The
compiler is written in Rust initially (`nova0`) and is progressively rewritten
in NavaScript itself. LLVM performs instruction selection, register allocation,
optimization, and native code generation; the compiler does not hand-write
ARM64 assembly.

```
NavaScript source (.ns, TypeScript-compatible syntax)
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
                 │  NavaScript IR    │   basic blocks, control flow
                 └─────────────┘
                        │
                        ▼
                 ┌─────────────┐
                 │ IR optimizer│   Phase 8 — not yet in the pipeline
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

The deep-module design of the front-end seam (what the driver calls, what is
hidden behind it, and which adapters are justified) is recorded in
`docs/DESIGN.md`. In short: `navascript_parser::parse(&str) -> ParseOutput` is the
external seam; `navascript-lexer` is a private internal seam; `navascript-ast` is a public
dependency-free leaf shared with the type checker and IR lowering.

## 2. Module responsibilities

| Module | Location | Responsibility | Allowed deps |
| --- | --- | --- | --- |
| Lexer | `compiler/lexer/` | Source → token stream with positions | none |
| Parser | `compiler/parser/` | Tokens → AST; syntax diagnostics | lexer, ast |
| AST | `compiler/ast/` | Node definitions, spans, debug printing | none |
| Symbol table | `compiler/symbol/` | Scopes, symbols, name resolution | ast |
| Semantic | `compiler/semantic/` | Reserved placeholder; scope building/validity checks live in the type checker (ADR-0010) | ast, symbol |
| Type checker | `compiler/typechecker/` | Types, inference, assignability, semantic checks | ast, symbol, semantic |
| IR | `compiler/ir/` | IR data model, lowering, verifier | typechecker, ast |
| Optimizer | `compiler/optimizer/` | Reserved placeholder; IR → IR passes arrive in Phase 8 | ir |
| Codegen | `compiler/codegen/` | IR → textual LLVM IR; no LLVM bindings (ADR-0004) | ir |
| Build | `compiler/driver/src/build.rs` | llc/clang invocation, host triple query | codegen, external tools |
| Driver | `compiler/driver/` | CLI, orchestration, link invocation | all above |
| Runtime | `runtime/` | Allocator, bigint helpers, strings, arrays, objects, console; closures/exceptions still pending in Phase 6 (ADR-0015) | codegen ABI contract |
| Tests | `tests/` | Tests for every public interface, plus future compatibility suites | — |

All tests live in the top-level `tests/` tree (ADR-0012); each module has a
corresponding test file there and is exercised only through its public
interface. A change to a module must be accompanied by its tests passing
(`cargo test --workspace`).

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
arrays and objects; runtime errors are fatal, so there is no exception ABI). This
contract is versioned in `runtime/README.md`, and exercised by
`tests/runtime/runtime_test.c` plus the end-to-end collection tests.

## 4. LLVM strategy

LLVM 23.1.6 is installed at `/opt/homebrew/opt/llvm` with AArch64 and X86
targets built. ADR-0004 resolved the integration choice in Phase 4; the options
considered were:

- **Option A — `llvm-sys` crate:** link libLLVM from Homebrew via
  `LLVM_SYS_231_PREFIX`. Tight integration, in-process passes, but version-
  sensitive build setup on macOS.
- **Option B — textual IR pipeline:** emit `.ll`, then call `llc`/`clang` from
  the driver. Simpler build, easier debugging, slower compile, more process
  plumbing.

**Decision:** Option B — emit `.ll`, run `llc -mtriple <host triple>`, link with
`clang`. The host triple comes from `llvm-config --host-target`; it is never
hardcoded to ARM64. The driver owns tool discovery and build errors.

## 5. Repository layout

```
compiler/     source modules + Cargo crates (workspace); each crate has one
              public façade (lib.rs) and private implementation modules
runtime/      navascript_runtime.h/.c plus memory/, bigint/, strings/, arrays/,
              objects/, console/ — one source directory per runtime concern
tests/        ast, diagnostics, lexer, parser, symbol, typechecker, ir, codegen,
              driver, runtime; compatibility added in Phase 7
docs/         this documentation set
examples/     sample .ns programs, starting with fibonacci.ns
scripts/      build/test/benchmark helper scripts
```

## 6. Build profiles

- **Debug (default):** LLVM IR is compiled with `llc -O0` and the C runtime with
  `clang -O0`. Source-level debug info is not emitted yet (Phase 11). `--emit-ir`
  prints the generated LLVM IR on demand; there are no always-on IR dumps.
- **Release (`--release`):** `llc -O3` and `clang -O2` for the C runtime. There is
  no separate `opt` pass pipeline or in-process LLVM pass manager (ADR-0004);
  dedicated IR optimization passes arrive in Phase 8.

## 7. Testing strategy

- All tests in `tests/`, one file per module's public interface; no inline
  test modules in `compiler/`. See `docs/BUILD.md` for the target list.
- Interface tests for the lexer and parser; golden-style output assertions for
  textual IR and generated LLVM IR.
- End-to-end tests compile `.ns` → executable → run → assert stdout/exit code.
- Compatibility corpus compares behavior against `tsc`/Node where applicable.
- Every stage of bootstrap runs the same test suite (see `BOOTSTRAP.md`).

## 8. Architectural invariants

1. LLVM is a backend detail, not a dependency of the front end.
2. No module claims a feature without a passing test.
3. IR is platform-independent; target specifics live in codegen.
4. The Rust compiler remains functional throughout self-hosting.
5. Diagnostics always carry source position information.
6. Unsupported TypeScript features are documented, never silently miscompiled.
