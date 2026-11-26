# NavaScript

A long-term compiler project: a language with TypeScript-compatible syntax that
compiles to optimized native machine code, with the goal of becoming
self-hosting.

**Status:** Phases 0–8 complete plus a pre-bootstrap audit. Phase 7 measured
compatibility against `tsc` + Node with a reproducible 32-case corpus, Phase 8
added IR optimization, and the audit fixed provable bugs and added typed
file/argv/string/array builtins. Phase 9 (Self-hosting) is next; see
`docs/PREBOOTSTRAP_AUDIT.md`.

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
| Python | 3.14.6 | for test and compatibility harness scripts |
| Node / TypeScript | 26.13.1 / 7.0.2 (`tsc`) | Phase 7 reference behaviour |
| CMake / Ninja | 4.5.1 / 1.13.2 — installed via Homebrew | available for future C++/CMake work; the current backend uses textual LLVM IR + `llc`/`clang` |

## Repository layout

```
compiler/    lexer, parser, ast, diagnostics, symbol, semantic, typechecker, ir,
             optimizer, codegen, driver
runtime/     navascript_runtime.h/.c plus memory/, bigint/, strings/, arrays/, objects/,
             console/ sources (one concern per directory)
tests/       ast, diagnostics, lexer, parser, symbol, typechecker, ir, codegen,
             driver, runtime, compatibility (32-case corpus), optimizer
benchmarks/  Phase 8 baseline/optimized programs
docs/        planning and tracking documents
examples/    sample programs (self_host_stdlib.ns exercises Phase 9 builtins)
scripts/     helper scripts
```

## Build and test

```bash
cargo build --workspace
cargo test --workspace     # 304 tests through Phase 8.1
python3 -m unittest discover -s tests/compatibility -p 'test_*.py'  # 8 harness tests
python3 -m unittest discover -s tests/benchmark -p 'test_*.py'      # 4 harness tests
python3 scripts/compat.py   # regenerate docs/COMPATIBILITY_REPORT.md (needs tsc + node)
python3 scripts/compat.py --check  # fail if the committed report is stale
python3 scripts/benchmark.py       # regenerate docs/BENCHMARKS.md (needs LLVM opt)
python3 scripts/benchmark.py --check  # re-measure deterministic benchmark facts
target/debug/navascript --version
target/debug/navascript --help
target/debug/navascript build examples/fibonacci.ns -o fibonacci
./fibonacci                                             # prints 55
target/debug/navascript build --release examples/fibonacci.ns -o fibonacci
target/debug/navascript build --emit-ir examples/fibonacci.ns   # print LLVM IR
```

Helper scripts: `scripts/build.sh`, `scripts/test.sh` (Rust + Python suites),
`scripts/compat.py` (compatibility corpus) and `scripts/benchmark.py` (Phase 8).
CI runs fmt, clippy, the full Rust suite, both Python harness suites and both
report `--check` gates on macOS (`.github/workflows/ci.yml`). Full build,
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
- Exceptions (`examples/exceptions.ns`) — `throw`/`try`/`catch`/`finally` with
   same-function handling; cross-function unwinding, abrupt control flow out of
   protected regions and unannotated catch bindings are diagnosed (ADR-0028).
- Async/await (`examples/async_await.ns`) — `async function` returns a promise
   value and `await` unwraps it synchronously; there is no event loop,
   microtask queue, `.then`/`.catch` or `Promise.all` (ADR-0029).

All Phase 6 expansion items are now delivered. Phase 7 adds the measured
compatibility report: `docs/COMPATIBILITY_REPORT.md` (19 executed cases pass,
13 documented skips, ADR-0030). Phase 8 adds IR folding/DCE/inlining and escape
analysis, LLVM `opt -passes=default<O3>` in release builds, and
`docs/BENCHMARKS.md` (108 → 104 LLVM IR instructions, 4/4 speedups, 4/4 matching
outputs, ADR-0031).

## Documentation

- **Start here:** `docs/PROGRESS.md` — central progress tracker and work log.
- `docs/BUILD.md` — prerequisites, build/test commands, CLI usage.
- `docs/ROADMAP.md` — 12 milestones with tasks, tests, acceptance criteria.
- `docs/ARCHITECTURE.md` — module boundaries and data flow.
- `docs/DESIGN.md` — deep-module interfaces and seams.
- `docs/LANGUAGE_SPEC.md` — supported language subset.
- `docs/COMPATIBILITY.md` — TypeScript compatibility matrix and unsupported-feature register.
- `docs/COMPATIBILITY_REPORT.md` — measured, reproducible Phase 7 results.
- `docs/BENCHMARKS.md` — Phase 8 baseline/optimized results.
- `docs/PREBOOTSTRAP_AUDIT.md` — audit findings, fixes and remaining long-term gaps.
- Phase 8.1 adds `any`, tagged unions, conservative GC and per-file diagnostics (ADR-0034).
- `docs/BOOTSTRAP.md` — three-stage self-hosting strategy.
- `docs/DECISIONS.md` — architecture decision records.

## Working rules

- One milestone or clearly scoped subtask at a time.
- Read existing code before modifying it.
- Tests for every implementation in `tests/`; run them after every change.
- Never claim a feature implemented unless tested.
- Document unsupported features; never silently miscompile TypeScript.
- Update `docs/PROGRESS.md` after every completed task.
