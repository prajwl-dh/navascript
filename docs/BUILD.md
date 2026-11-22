# Nava — Build and Test Guide

## 1. Prerequisites (verified on this machine)

| Tool | Version used | Purpose |
| --- | --- | --- |
| macOS | 27.2.1, `arm64` | development and first release target |
| Rust / Cargo | 1.100.0 (Homebrew) | bootstrap compiler (`nova0`) |
| LLVM | 23.1.6 at `/opt/homebrew/opt/llvm` | optimization + native codegen (Phase 4+) |
| Clang | Apple clang 21 (Command Line Tools) | native toolchain integration |
| Git | 2.54.0 | version control |
| Python 3 | 3.14.6 | test and benchmark scripts |
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
| `tests/driver/` | CLI argument parsing and the real `nava` binary |
| `tests/ir/` | lowering, verifier rejection cases, and textual IR output |
| `tests/codegen/` | Nava IR → LLVM IR translation |
| `tests/driver/native.rs` | end-to-end native compile + run (fibonacci, strings, arrays, objects, loops, `break`/`continue`, compound/updates/power/ternary, arrows, optional chaining, bigint, optional calls, dynamic indices, runtime errors, debug/release) |
| `tests/runtime/runtime_test.c` | C runtime unit tests, compiled and run by `tests/runtime/runtime.rs` |

`tests/runtime/` (C runtime unit tests) landed in Phase 5; `tests/compatibility/`
is reserved for the Phase 7 corpus. `tests/codegen/` and `tests/driver/native.rs`
landed in Phase 4.

Cargo only discovers `tests/` inside a package root, so every file is
registered as an explicit `[[test]]` target in `compiler/driver/Cargo.toml`.
Tests import the crates they exercise through `[dev-dependencies]`. Moving a
test out of a crate means it can only use that crate's public interface — which
is the intended contract. `.github/workflows/ci.yml` runs fmt, clippy and the
full test suite on macOS.

## 4. CLI usage

```bash
cargo run -p nava -- --version
cargo run -p nava -- --help
cargo run -p nava -- build examples/fibonacci.ts
```

After `cargo build --workspace`, the binary is at `target/debug/nava`:

```bash
target/debug/nava --version     # nava 0.1.0
target/debug/nava --help
target/debug/nava build examples/fibonacci.ts -o fibonacci
./fibonacci                     # prints 55
target/debug/nava build --release examples/fibonacci.ts -o fibonacci
target/debug/nava build --emit-ir examples/fibonacci.ts   # LLVM IR
```

`nava build` parses, type-checks, lowers and compiles the source to a native
Mach-O executable. `-o` sets the output path (default: source file stem),
`--release` enables `-O3`/`-O2` (debug uses `-O0`; source-level debug info is
Phase 11), and `--emit-ir` prints LLVM IR instead of linking. The external
toolchain is discovered from `LLVM_SYS_231_PREFIX` (or
`/opt/homebrew/opt/llvm`): `llvm-config` supplies the host target triple,
`llc` emits the object, and `clang` compiles the small C runtime and links.
No CMake or Ninja is required.

Tests that need the native toolchain call
`nava_driver::build::native_tools_available()` and skip cleanly when it is
absent, so `cargo test --workspace` passes on machines without LLVM. CI installs
LLVM with `brew install llvm` and exports the discovered prefix so the native
tests actually execute there.

## 5. Workspace layout

```
Cargo.toml                  workspace root, shared package metadata
compiler/
  ast/ diagnostics/ lexer/ parser/ symbol/ semantic/ typechecker/
  ir/ optimizer/ codegen/
  driver/                   package `nava`: lib `nava_driver` + bin `nava`

runtime/
  nava_runtime.h/.c         ABI header and process entry point
  memory/ bigint/ strings/ arrays/ objects/ console/   one source per concern
```

The front end (lexer → AST → type checker → IR) must not depend on LLVM. Only
`compiler/codegen` may depend on it.

## 6. LLVM integration (resolved in Phase 4)

ADR-0004 selected the textual-IR route: `nava-codegen` emits `.ll`, the driver
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
- Closures, function values and exceptions are still pending Phase 6 work
  (ADR-0015; see `PROGRESS.md`); runtime bounds/field errors terminate the
  process with a diagnostic.
- Loops support `while`, `do-while`, `for` and `for-of` over arrays/strings,
  with `break`/`continue` for the innermost loop; general iterables are not
  implemented.
- `bigint` is a signed 64-bit integer, not arbitrary precision: mixing it with
  `number` is diagnosed, `+ - *` wrap on overflow, and div/mod/pow are checked;
  optional calls lower as direct calls until nullable callables exist.
- Only the detected host target is validated; x86-64 and other targets are
  Phase 11.
- Debug builds are unoptimized (`-O0`); source-level debug info is Phase 11.
- CMake 4.5.1 and Ninja 1.13.2 are installed but not required by the chosen
  LLVM integration.
