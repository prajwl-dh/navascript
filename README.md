# NavaScript

A long-term compiler project: a language with TypeScript-compatible syntax that
compiles to optimized native machine code, with the goal of becoming
self-hosting.

**Status:** Phase 6 (Language expansion) in progress — loops (with
`break`/`continue`), the additional literals/operators, arrow functions,
optional chaining, bigint literals, optional calls, generics, classes with
inheritance, unions/intersections, full structural typing, modules and
closures/higher-order functions are complete; exceptions and async/await are
pending.

## Goals (tracked independently)

1. **TypeScript compatibility** — familiar TypeScript syntax and semantics.
2. **Native compilation** — native ARM64 macOS executables via LLVM, no Node.js
   at runtime.
3. **Self-hosting** — the compiler is eventually written in NavaScript itself.

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
| CMake / Ninja | 4.5.1 / 1.13.2 — installed via Homebrew | available for future C++/CMake work; the current backend uses textual LLVM IR + `llc`/`clang` |

## Repository layout

```
compiler/    lexer, parser, ast, diagnostics, symbol, semantic, typechecker, ir,
             optimizer, codegen, driver
runtime/     navascript_runtime.h/.c plus memory/, bigint/, strings/, arrays/, objects/,
             console/ sources (one concern per directory)
tests/       ast, diagnostics, lexer, parser, symbol, typechecker, ir, codegen,
             driver, runtime (compatibility added in Phase 7)
docs/        planning and tracking documents
examples/    sample programs
scripts/     helper scripts
```

## Build and test

```bash
cargo build --workspace
cargo test --workspace     # 258 tests through Phase 6 (loops, operators, arrows, optional chaining, bigint, optional calls, generics, classes, unions, modules, closures)
target/debug/navascript --version
target/debug/navascript --help
target/debug/navascript build examples/fibonacci.ns -o fibonacci
./fibonacci                                             # prints 55
target/debug/navascript build --release examples/fibonacci.ns -o fibonacci
target/debug/navascript build --emit-ir examples/fibonacci.ns   # print LLVM IR
```

Helper scripts: `scripts/build.sh` and `scripts/test.sh`. CI runs the same
fmt/clippy/test checks on macOS (`.github/workflows/ci.yml`). Full build,
test and LLVM toolchain instructions are in `docs/BUILD.md`.

Phases 0–5 are complete: the CLI parses, type-checks, lowers to NavaScript IR, emits
LLVM IR, and links a native executable. The runtime provides arena-allocated
strings, arrays and objects with bounds checking. The first major milestone —
fibonacci printing `55` without Node.js — is verified by
`tests/driver/native.rs`. Phase 6 is under way and has delivered:

- Loops: `while`, `do-while`, C-style `for` and `for-of` over arrays/strings,
  with `break`/`continue` (`examples/loops.ns`, `examples/for_of.ns`,
  `examples/break_continue.ns`).
- The additional literals/operators: compound assignment (`+= -= *= /= %=`),
  `++`/`--`, `**`, ternary, hex/binary/octal literals with `_` separators,
  Unicode identifiers and `\x`/`\u` escapes (`examples/operators.ns`).
- Arrow functions in any expression position with annotated parameters:
  function values, captured bindings and higher-order calls
  (`examples/arrows.ns`, `examples/closures.ns`; ADR-0027).
- Optional chaining `a?.b` / `a?.[i]` for reference-typed results
  (`examples/optional.ns`).

- Bigint literals (`123n`, `0xffn`, `1_000n`) as a `bigint` type backed by a
   signed 64-bit integer, with integer arithmetic, comparisons, `++`/`--`,
   bigint arrays/objects and `console.log` (`examples/bigint.ns`).
- Optional calls `a?.()` (`examples/optional_call.ns`) — parsed, type-checked
   and lowered as a direct call for named functions or an indirect call through
   a function value; the null branch is unobservable until NavaScript has
   nullable callable values.
- Generics (`examples/generics.ns`) — generic functions and interfaces with
   inferred or explicit type arguments, lowered by monomorphization.
- Classes and inheritance (`examples/classes.ns`) — fields, constructors,
   methods, `this`, `super(...)`, `extends`, inherited members and constructors;
   methods lower to named functions with static dispatch (ADR-0024).
- Unions, intersections, narrowing and optional properties
   (`examples/unions.ns`) — nullable reference unions with control-flow
   narrowing, merged intersection layouts, optional object fields (ADR-0025).
- Modules (`examples/modules.ns` + `examples/math.ns`) — `import`/`export`
   syntax and import-graph resolution from the entry file (ADR-0026).
- Closures and higher-order functions (`examples/closures.ns`) — first-class
   function values, shared captured bindings, nested closures, named functions
   passed as values and indirect calls through the closure ABI (ADR-0027).

Still pending in Phase 6: exceptions (deferred from Phase 5 by ADR-0015) and
async/await.

## Documentation

- **Start here:** `docs/PROGRESS.md` — central progress tracker and work log.
- `docs/BUILD.md` — prerequisites, build/test commands, CLI usage.
- `docs/ROADMAP.md` — 12 milestones with tasks, tests, acceptance criteria.
- `docs/ARCHITECTURE.md` — module boundaries and data flow.
- `docs/DESIGN.md` — deep-module interfaces and seams.
- `docs/LANGUAGE_SPEC.md` — supported language subset.
- `docs/COMPATIBILITY.md` — TypeScript compatibility matrix.
- `docs/BOOTSTRAP.md` — three-stage self-hosting strategy.
- `docs/DECISIONS.md` — architecture decision records.

## Working rules

- One milestone or clearly scoped subtask at a time.
- Read existing code before modifying it.
- Tests for every implementation in `tests/`; run them after every change.
- Never claim a feature implemented unless tested.
- Document unsupported features; never silently miscompile TypeScript.
- Update `docs/PROGRESS.md` after every completed task.
