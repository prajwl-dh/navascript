# Nava — TypeScript Compatibility Matrix

This matrix is the authoritative record of what Nava supports relative to
TypeScript. A feature is only marked supported when a test proves it.

**Legend:** `[x]` supported + tested · `[~]` partial · `[ ]` not started ·
`[-]` intentionally unsupported / divergent (with note)

Last updated: 2026-11-21 — Phase 1 syntax, Phase 2 type-system and Phase 3 IR
entries are implemented and tested; runtime entries remain unimplemented.

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
| Interfaces | [ ] | — | Phase 2/6 |
| Type aliases | [ ] | — | Phase 2 |
| Unions / intersections | [ ] | — | Phase 6 |
| Modules (`import`/`export`) | [ ] | — | Phase 6/7 |
| Async/await, promises | [ ] | — | Phase 6 |
| Decorators | [-] | — | Deferred; not in initial scope |
| JSX / `.tsx` | [-] | — | Deferred |

## 2. Type system

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| `number`, `string`, `boolean`, `void` | [x] | `tests/typechecker/` | Phase 2 |
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
| Arithmetic / comparisons | [ ] | — | Phase 4 |
| Recursion | [ ] | — | Required for fibonacci |
| Strings | [ ] | — | Phase 5 |
| Arrays & objects | [ ] | — | Phase 5 |
| Closures / function values | [ ] | — | Phase 5/6 |
| Exceptions | [ ] | — | Phase 5; semantics will differ initially |
| `console.log` | [~] | `tests/ir/` | typed + lowered as a call; runtime implementation Phase 5 |
| Memory management | [ ] | — | Strategy in ADR-0005 |

## 4. Platform

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| macOS ARM64 native executable | [ ] | — | Phase 4; host is arm64 |
| x86-64 macOS | [ ] | — | Later; LLVM target available |
| Linux / Windows | [-] | — | Phase 11 |
| Debug info / debugger | [ ] | — | Phase 11 |
| No Node.js dependency at runtime | [ ] | — | Core requirement |

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
| Mixed string/number `+` coercion | accepted as JavaScript-style concatenation (`"a" + 1` is `string`); documented difference, not a TypeScript parity claim | Phase 7 |
| Collection literals in IR | type-check but lowering reports "not supported in IR yet" | Phase 5 |
| Indirect calls in IR | only direct named calls lower; member calls are limited to the known builtin `console.log`, and unknown ones are diagnosed | Phase 5 |
| Functions falling through without returning | lowering reports "may fall through without returning" instead of fabricating a value | Phase 5 |
