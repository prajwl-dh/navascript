# NavaScript

A TypeScript-compatible language that compiles to native ARM64 macOS
executables, with the long-term goal of self-hosting.

**Status:** Phases 0–8 and 8.1 are complete; Phase 9 (compiler rewrite in
NavaScript) is next. See `docs/PROGRESS.md` for evidence and `docs/ROADMAP.md`
for what each phase means.

## Goals (tracked independently)

1. TypeScript compatibility
2. Native compilation (no Node.js at runtime)
3. Self-hosting

Progress on one goal is not progress on another.

## Quick start

```bash
cargo build --workspace
cargo test --workspace
target/debug/navascript build examples/fibonacci.ns -o fibonacci
./fibonacci                                  # prints 55
```

Prerequisites, CLI options, test surfaces, harness commands and CI details are
in `docs/BUILD.md`.

## Documentation map

| Question | Canonical document |
| --- | --- |
| What is planned and what does "done" mean? | `docs/ROADMAP.md` |
| What is the current state and why? | `docs/PROGRESS.md` |
| What language features exist? | `docs/LANGUAGE_SPEC.md` |
| What is supported vs unsupported vs measured? | `docs/COMPATIBILITY.md`, `docs/COMPATIBILITY_REPORT.md` |
| What were the optimization results? | `docs/BENCHMARKS.md` |
| How do modules and the pipeline fit together? | `docs/ARCHITECTURE.md`, `docs/DESIGN.md` |
| Why was something designed this way? | `docs/DECISIONS.md` |
| How do I build, test and use the CLI? | `docs/BUILD.md` |
| What is the runtime ABI? | `runtime/README.md` |

## Working rules

- One milestone or clearly scoped subtask at a time.
- Tests through public interfaces in `tests/`; run them after every change.
- Never claim a feature implemented unless a test proves it.
- Document unsupported features; never silently miscompile TypeScript.
- Keep `docs/PROGRESS.md` and `docs/COMPATIBILITY.md` current; do not duplicate
  their content in other documents.
