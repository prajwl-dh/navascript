# NavaScript — Build and Test Guide

## 1. Prerequisites (verified on this machine)

| Tool | Version used | Purpose |
| --- | --- | --- |
| macOS | 27.2.1, `arm64` | development and first release target |
| Rust / Cargo | 1.100.0 (Homebrew) | bootstrap compiler (`nova0`) |
| LLVM | 23.1.6 at `/opt/homebrew/opt/llvm` | optimization + native codegen (Phase 4+) |
| Clang | Apple clang 21 (Command Line Tools) | native toolchain integration |
| Git | 2.54.0 | version control |
| Python 3 | 3.14.6 | test and compatibility harness scripts |
| Node.js / TypeScript | 26.13.1 / 7.0.2 (`tsc`) | Phase 7 reference behaviour for the compatibility corpus |
| CMake | 4.5.1 (Homebrew) | available for future CMake/C++ builds |
| Ninja | 1.13.2 (Homebrew) | available as a CMake generator |

Rust was installed via Homebrew, **not** rustup, so `rustup` commands are not
available. LLVM's tools live in `/opt/homebrew/opt/llvm/bin` (`llc`, `opt`,
`llvm-as`, `llvm-dis`, `llvm-link`, `llvm-config`) and are not on `PATH` by
default.

CMake 4.5.1 and Ninja 1.13.2 are installed via Homebrew as of 2026-11-21. They
are **not required** by the current build: the Phase 4 backend emits textual
LLVM IR and calls `llc`/`clang` directly (ADR-0004). They are available for a
future `llvm-sys` integration or C++ tooling.

Verify the environment at any time:

```bash
uname -m
rustc --version && cargo --version
/opt/homebrew/opt/llvm/bin/llvm-config --version --host-target
clang --version
cmake --version && ninja --version   # optional tooling, installed
```

## 2. Build

```bash
cargo build --workspace
```

Or use the helper script:

```bash
scripts/build.sh
```

Debug artifacts go to `target/debug/`; release artifacts to `target/release/`.

## 3. Test

```bash
cargo test --workspace
```

Or:

```bash
scripts/test.sh
```

**All tests live in the top-level `tests/` tree** — no `#[cfg(test)]` modules
remain in `compiler/`. Each test target calls only public interfaces:

| Directory | Tests |
| --- | --- |
| `tests/ast/` | span behaviour |
| `tests/diagnostics/` | positions, dedup, ordering |
| `tests/lexer/` | tokenization via `tokenize` |
| `tests/parser/` | parsing via `parse`, plus `pipeline.rs` (parse seam from outside) |
| `tests/symbol/` | scopes, shadowing, type display |
| `tests/typechecker/` | type resolution, checking, plus `pipeline.rs` |
| `tests/driver/` | CLI argument parsing and the real `navascript` binary |
| `tests/ir/` | lowering, verifier rejection cases, and textual IR output |
| `tests/codegen/` | NavaScript IR → LLVM IR translation |
| `tests/driver/native.rs` | end-to-end native compile + run (fibonacci, strings, arrays, objects, loops, `break`/`continue`, compound/updates/power/ternary, arrows, optional chaining, bigint, optional calls, generics, classes/inheritance, unions/intersections, modules, closures/higher-order functions with shared
captures, exceptions, async/await, dynamic indices, runtime errors, debug/release) |
| `tests/runtime/runtime_test.c` | C runtime unit tests, compiled and run by `tests/runtime/runtime.rs` |
| `tests/compatibility/corpus.rs` | corpus integrity; every `match` case reaches valid IR; report coverage |
| `tests/compatibility/test_compat.py` | harness validation, classification, pinned divergences, deterministic rendering, CLI façade |
| `tests/optimizer/{optimize,inline,blocks,native}.rs` | fold/DCE, inlining with renamed results, escape analysis, branch folding, unreachable blocks, baseline/optimized output equivalence |
| `tests/benchmark/test_benchmark.py` | instruction counting, derived metrics, deterministic report rendering, CLI façade |

`tests/runtime/` (C runtime unit tests) landed in Phase 5; `tests/compatibility/`
landed in Phase 7 with the tsc/Node corpus; `tests/optimizer/` landed in Phase 8.
`tests/codegen/` and `tests/driver/native.rs` landed in Phase 4. The runners are
in `scripts/`:

```bash
cargo build --workspace
python3 scripts/compat.py                    # regenerate docs/COMPATIBILITY_REPORT.md
python3 scripts/compat.py --check            # fail if the committed report is stale
python3 -m unittest discover -s tests/compatibility -p 'test_*.py'

python3 scripts/benchmark.py                 # regenerate docs/BENCHMARKS.md
python3 scripts/benchmark.py --check         # re-measure deterministic facts
python3 -m unittest discover -s tests/benchmark -p 'test_*.py'
```

The compatibility runner needs `tsc` and `node` on `PATH`; the benchmark runner
needs the LLVM `opt` tool as well as `llc` and `clang`.

Cargo only discovers `tests/` inside a package root, so every file is
registered as an explicit `[[test]]` target in `compiler/driver/Cargo.toml`.
Tests import the crates they exercise through `[dev-dependencies]`. Moving a
test out of a crate means it can only use that crate's public interface — which
is the intended contract. `.github/workflows/ci.yml` runs fmt, clippy, the full
Rust suite, both Python harness test sets and both `--check` reproducibility
gates on macOS (installing TypeScript for the corpus). `scripts/test.sh` runs
the Rust suite plus the Python harness tests locally; the corpus and benchmark
runs are separate commands because they invoke `tsc`, Node and LLVM.

## 4. CLI usage

```bash
cargo run -p navascript -- --version
cargo run -p navascript -- --help
cargo run -p navascript -- build examples/fibonacci.ns
```

After `cargo build --workspace`, the binary is at `target/debug/navascript`:

```bash
target/debug/navascript --version     # navascript 0.1.0
target/debug/navascript --help
target/debug/navascript build examples/fibonacci.ns -o fibonacci
./fibonacci                     # prints 55
target/debug/navascript build --release examples/fibonacci.ns -o fibonacci
target/debug/navascript build --emit-ir examples/fibonacci.ns   # final LLVM IR
target/debug/navascript build --emit-ir --release examples/fibonacci.ns
target/debug/navascript build --no-opt examples/fibonacci.ns -o fibonacci-baseline
target/debug/navascript build examples/modules.ns -o modules    # resolves imports
```

`navascript build` parses, type-checks, lowers, optimizes and compiles the source
to a native Mach-O executable. `-o` sets the output path (default: source file
stem). Debug applies IR folding/DCE with `llc -O0`; `--release` adds inlining and
escape analysis, then runs `opt -passes=default<O3>`, `llc -O3` and `clang -O2`
(source-level debug info is Phase 11). `--no-opt` skips all IR and LLVM passes
for benchmark baselines. `--emit-ir` prints the final LLVM IR instead of linking;
with `--release` it includes the `opt` pipeline. The external toolchain is
discovered from `LLVM_SYS_231_PREFIX` (or `/opt/homebrew/opt/llvm`):
`llvm-config` supplies the host target triple, `opt` runs the release pipeline,
`llc` emits the object, and `clang` compiles the small C runtime and links.
`opt` is required only for optimized builds. No CMake or Ninja is required.

Tests that need the native toolchain call
`navascript_driver::build::native_tools_available()` and skip cleanly when it is
absent, so `cargo test --workspace` passes on machines without LLVM. CI installs
LLVM with `brew install llvm` and exports the discovered prefix so the native
tests actually execute there.

## 5. Workspace layout

```
Cargo.toml                  workspace root, shared package metadata
compiler/
  ast/ diagnostics/ lexer/ parser/ symbol/ semantic/ typechecker/
  ir/ optimizer/ codegen/
  driver/                   package `navascript`: lib `navascript_driver` + bin `navascript`

runtime/
  navascript_runtime.h/.c         ABI header and process entry point
  memory/ bigint/ strings/ arrays/ objects/ console/   one source per concern
```

The front end (lexer → AST → type checker → IR) must not depend on LLVM. Only
`compiler/codegen` may depend on it.

## 6. LLVM integration (resolved in Phase 4)

ADR-0004 selected the textual-IR route: `navascript-codegen` emits `.ll`, the driver
runs `llc -filetype=obj -mtriple <triple>`, then `clang` compiles the runtime
shim and links the executable. Override the tool location with:

```bash
export LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm
```

The target triple is queried via `llvm-config --host-target`; ARM64 is never
hardcoded. Errors from `llvm-config`, `llc` or `clang` are reported with exit
code 1.

## 7. Current limitations

- Runtime memory is a program-lifetime bump arena; nothing is reclaimed until
  process exit (ADR-0005).
- Exceptions work within one function (ADR-0028): `throw` targets the innermost
  enclosing catch and body/catch converge through `finally`. Cross-function
  throws, abrupt control flow out of protected regions and unannotated catch
  bindings are diagnosed. Runtime bounds/field errors still terminate the
  process. Closures and function values are supported: captured bindings are
  shared cells and calls through a value use an indirect-call ABI (ADR-0027).
- Async/await is synchronous (ADR-0029): promises wrap a value in an object
  slot and `await` reads it without suspending. There is no event loop or
  promise combinators; `await` in a non-async function is diagnosed.
- Loops support `while`, `do-while`, `for` and `for-of` over arrays/strings,
  with `break`/`continue` for the innermost loop; general iterables are not
  implemented.
- `bigint` is a signed 64-bit integer, not arbitrary precision: mixing it with
  `number` is diagnosed, `+ - *` wrap on overflow, and div/mod/pow are checked;
  optional calls lower as direct calls for named functions and indirect calls
  through function values until nullable callables exist.
- Only the detected host target is validated; x86-64 and other targets are
  Phase 11.
- Debug builds apply IR folding/DCE only and use `llc -O0`; source-level debug
  info is Phase 11. The optimization pass set and benchmark results are in
  `docs/BENCHMARKS.md` (ADR-0031).
- CMake 4.5.1 and Ninja 1.13.2 are installed but not required by the chosen
  LLVM integration.
