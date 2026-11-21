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

Rust was installed via Homebrew, **not** rustup, so `rustup` commands are not
available. LLVM's tools live in `/opt/homebrew/opt/llvm/bin` (`llc`, `opt`,
`llvm-as`, `llvm-dis`, `llvm-link`, `llvm-config`) and are not on `PATH` by
default.

Missing tools, install when a phase needs them:

```bash
brew install cmake ninja
```

Verify the environment at any time:

```bash
uname -m
rustc --version && cargo --version
/opt/homebrew/opt/llvm/bin/llvm-config --version --host-target
clang --version
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
| `tests/driver/native.rs` | end-to-end native compile + run (debug and release) |

`tests/codegen/`, `tests/runtime/` and `tests/compatibility/` are reserved for
Phase 4, 5 and 7 suites.

Cargo only discovers `tests/` inside a package root, so every file is
registered as an explicit `[[test]]` target in `compiler/driver/Cargo.toml`.
Tests import the crates they exercise through `[dev-dependencies]`. Moving a
test out of a crate means it can only use that crate's public interface — which
is the intended contract. `.github/workflows/ci.yml` runs fmt, clippy and the
full test suite on macOS.

## 4. CLI usage (Phase 0)

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

## 5. Workspace layout

```
Cargo.toml                  workspace root, shared package metadata
compiler/
  ast/ lexer/ parser/ symbol/ semantic/ typechecker/
  ir/ optimizer/ codegen/
  driver/                   package `nava`: lib `nava_driver` + bin `nava`
```

The front end (lexer → AST → type checker → IR) must not depend on LLVM. Only
`compiler/codegen` may depend on it.

## 6. LLVM integration (Phase 4+)

The integration method is recorded as **Proposed** in `docs/DECISIONS.md`
(ADR-0004). If the `llvm-sys` route is chosen, set:

```bash
export LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm
```

If the textual-IR route is chosen, add `/opt/homebrew/opt/llvm/bin` to `PATH`.
Do not hardcode ARM64: query the target triple and let LLVM select the backend.

## 7. Known Phase 0 limitations

- `nava build` does not parse, type-check, or generate code.
- LLVM is not linked and no object files are produced.
- CMake and Ninja are not installed (not needed until deeper LLVM/C++ work).
