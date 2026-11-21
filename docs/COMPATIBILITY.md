# Nava — TypeScript Compatibility Matrix

This matrix is the authoritative record of what Nava supports relative to
TypeScript. A feature is only marked supported when a test proves it.

**Legend:** `[x]` supported + tested · `[~]` partial · `[ ]` not started ·
`[-]` intentionally unsupported / divergent (with note)

Last updated: 2026-11-21 — Phases 0–5 are complete. Syntax, type-system, IR,
codegen and native-execution entries are implemented and tested. The runtime
now provides strings, arrays, objects and fatal error reporting; closures and
exceptions are deferred to Phase 6 (ADR-0015).

## 1. Syntax

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| Lexical keywords & tokens | [x] | `tests/lexer/` | Phase 1 |
| Variables (`let`/`const`/`var`) | [x] | `tests/parser/` | Phase 1 |
| Functions & returns | [x] | `tests/parser/` | Phase 1 |
| Expressions & operator precedence | [x] | `tests/parser/` | Phase 1 |
| Blocks & lexical scope | [x] | `tests/parser/` + `tests/typechecker/` | AST blocks parsed; scopes and shadowing checked in Phase 2 |
| `if`/`else` | [x] | `tests/parser/` | Phase 1 |
| Loops (`for`/`while`) | [ ] | — | Phase 6 |
| Comments | [x] | `tests/lexer/` | Phase 1 |
| Diagnostics with line/column | [x] | `tests/parser/` + `tests/driver/` | caret rendering included |
| Classes | [ ] | — | Phase 6 |
| Generics | [ ] | — | Phase 6 |
| Interfaces | [x] | `tests/parser/` + `tests/typechecker/` | parsed and type-checked in Phase 2; full structural typing Phase 6 |
| Type aliases | [x] | `tests/parser/` + `tests/typechecker/` | chains and cycles handled in Phase 2 |
| Unions / intersections | [ ] | — | Phase 6 |
| Modules (`import`/`export`) | [ ] | — | Phase 6/7 |
| Async/await, promises | [ ] | — | Phase 6 |
| Decorators | [-] | — | Deferred; not in initial scope |
| JSX / `.tsx` | [-] | — | Deferred |

## 2. Type system

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| `number`, `string`, `boolean`, `void` | [x] | `tests/typechecker/` | Phase 2 |
| `null` | [~] | manual check | type-checks; `console.log(null)` prints `null`, but an inferred `null` value cannot be lowered (opaque type) |
| Arrays | [x] | `tests/typechecker/` | `T[]` and array literals |
| Object types | [x] | `tests/typechecker/` | object literals and member access |
| Function signatures | [x] | `tests/typechecker/` | `(a: T) => R`, arity and argument checks |
| Type inference | [x] | `tests/typechecker/` | variables and return types |
| Assignability checks | [x] | `tests/typechecker/` | assignments, arguments, returns |
| Structural typing | [~] | `tests/typechecker/` | basic object field checking; full structural typing Phase 6 |
| Generics | [ ] | — | Phase 6 |
| `any` / `unknown` | [ ] | — | Restricted semantics TBD |
| Conditional / mapped types | [-] | — | Long-term |

## 3. Runtime behavior

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| Arithmetic / comparisons | [x] | `tests/driver/native.rs`, `tests/codegen/` | native `fadd/fsub/fmul/fdiv/frem`, `fcmp`/`icmp`; boolean results print correctly |
| Recursion | [x] | `tests/driver/native.rs` | fibonacci calls itself natively and prints `55` |
| Strings | [x] | `tests/driver/native.rs` | literal, `+`, `length`, indexing, `==`, `console.log` |
| Arrays & objects | [x] | `tests/driver/native.rs` | literals, get/set, `length`, runtime bounds checks |
| Closures / function values | [ ] | — | Phase 6 (ADR-0015) |
| Exceptions | [ ] | — | Phase 6 (ADR-0015); runtime errors are currently fatal |
| `console.log` | [~] | `tests/driver/native.rs` | numbers, booleans and strings print natively; object/array values are not meaningfully supported (raw bytes printed) until Phase 6 |
| Memory management | [x] | runtime arena | program-lifetime bump arena, ADR-0005; no reclamation |

## 4. Platform

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| macOS ARM64 native executable | [x] | `tests/driver/native.rs` | host triple detected via llvm-config |
| x86-64 macOS | [ ] | — | LLVM target available; not yet validated |
| Linux / Windows | [-] | — | Phase 11 |
| Debug info / debugger | [ ] | — | Phase 11 |
| No Node.js dependency at runtime | [x] | `tests/driver/native.rs` | native Mach-O, C runtime shim only |

## 5. Compatibility testing strategy (Phase 7)

- Build a corpus of small TypeScript programs with expected stdout/exit code.
- Run each through `tsc` + Node (reference behavior) and through Nava.
- Classify results: pass / fail / skip (unsupported) / known-difference.
- Commit the report with counts and links to failing cases.
- Re-run on every milestone touching semantics; regressions must be investigated.

## 6. Unsupported features register

Maintain this list so gaps are explicit and never silently miscompiled:

| Feature | Reason | Planned phase |
| --- | --- | --- |
| Decorators | scope control | deferred |
| JSX / TSX | no DOM/UI target | deferred |
| Conditional / mapped types | high complexity | long-term |
| Full `lib.d.ts` | large surface | long-term |
| Node.js interop beyond documented subset | platform scope | Phase 7 best effort |
| `eval` / dynamic code | incompatible with native model | not planned |
| Arrow function expressions | `=>` is lexed and used in function *type annotations*, but arrow expressions are not parsed | Phase 6 |
| Hex / binary / octal / bigint literals | lexer accepts decimal digits only | Phase 6 |
| Compound operators (`+=`, `++`, `**`) and optional chaining (`?.`) | lexed as separate tokens, not parsed | Phase 6 |
| Non-ASCII identifiers | lexer accepts ASCII letters, `_`, `$` only | Phase 6 |
| `\u` / `\x` string escapes | only `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\'` decoded | Phase 6 |
| `strictFunctionTypes` parameter contravariance | function parameters are checked covariantly | Phase 6 |
| Mixed string/number `+` coercion | supported in Phase 5 via runtime number-to-string conversion | Phase 7 (full JS coercion rules) |
| Indirect calls in IR | only direct named calls lower; member calls are limited to the known builtin `console.log`, and unknown ones are diagnosed | Phase 6 |
| Garbage collection | allocated runtime objects live until process exit (ADR-0005) | Phase 8+ |
| Closures and first-class functions | no function values or captured environments in IR | Phase 6 |
| `throw` / `try` / `catch` / `finally` | not parsed; runtime errors terminate the process | Phase 6 |
