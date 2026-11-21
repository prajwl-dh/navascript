# Nava

A long-term compiler project: a language with TypeScript-compatible syntax that
compiles to optimized native machine code, with the goal of becoming
self-hosting.

**Status:** Planning complete. Implementation starts at Phase 0 (Foundation).

## Goals (tracked independently)

1. **TypeScript compatibility** — familiar TypeScript syntax and semantics.
2. **Native compilation** — native ARM64 macOS executables via LLVM, no Node.js
   at runtime.
3. **Self-hosting** — the compiler is eventually written in Nava itself.

Do not treat progress on one goal as progress on another.

## First major milestone

```typescript
function fibonacci(n: number): number {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
console.log(fibonacci(10)); // prints 55
```

Compile to a native macOS executable that prints `55` without Node.js.

## Development environment (verified)

| Tool | Version | Notes |
| --- | --- | --- |
| macOS | 27.2.1, arm64 | Apple Silicon |
| Rust / Cargo | 1.100.0 | Homebrew; no `rustup` |
| LLVM | 23.1.6 | `/opt/homebrew/opt/llvm`, AArch64 + X86 |
| Clang | Apple clang 21 | Command Line Tools |
| Git | 2.54.0 | configured |
| Python | 3.14.6 | for test/benchmark scripts |
| CMake / Ninja | missing | install with `brew install cmake ninja` when needed |

## Repository layout

```
compiler/    lexer, parser, ast, symbol, semantic, typechecker, ir,
             optimizer, codegen, driver
runtime/     memory, strings, arrays, objects, exceptions
bootstrap/   stage0 (Rust), stage1, stage2
stdlib/      Nava standard library
tests/       lexer, parser, typechecker, codegen, runtime, compatibility
benchmarks/  performance suite
docs/        planning and tracking documents
examples/    sample programs
scripts/     helper scripts
```

## Documentation

- **Start here:** `docs/PROGRESS.md` — central progress tracker and work log.
- `docs/ROADMAP.md` — 12 milestones with tasks, tests, acceptance criteria.
- `docs/ARCHITECTURE.md` — module boundaries and data flow.
- `docs/LANGUAGE_SPEC.md` — supported language subset.
- `docs/COMPATIBILITY.md` — TypeScript compatibility matrix.
- `docs/BOOTSTRAP.md` — three-stage self-hosting strategy.
- `docs/DECISIONS.md` — architecture decision records.

## Working rules

- One milestone or clearly scoped subtask at a time.
- Read existing code before modifying it.
- Tests alongside every implementation; run them after every change.
- Never claim a feature implemented unless tested.
- Document unsupported features; never silently miscompile TypeScript.
- Update `docs/PROGRESS.md` after every completed task.
