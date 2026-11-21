# Nava — TypeScript Compatibility Matrix

This matrix is the authoritative record of what Nava supports relative to
TypeScript. A feature is only marked supported when a test proves it.

**Legend:** `[x]` supported + tested · `[~]` partial · `[ ]` not started ·
`[-]` intentionally unsupported / divergent (with note)

Last updated: 2026-11-20 — Phase 1 syntax entries implemented and tested; all
type-system and runtime entries remain unimplemented.

## 1. Syntax

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| Lexical keywords & tokens | [x] | `nava-lexer` unit tests | Phase 1 |
| Variables (`let`/`const`/`var`) | [x] | `nava-parser` tests | Phase 1 |
| Functions & returns | [x] | `nava-parser` tests | Phase 1 |
| Expressions & operator precedence | [x] | `nava-parser` tests | Phase 1 |
| Blocks & lexical scope | [~] | `nava-parser` tests | AST blocks parsed; scope resolution is Phase 2 |
| `if`/`else` | [x] | `nava-parser` tests | Phase 1 |
| Loops (`for`/`while`) | [ ] | — | Phase 6 |
| Comments | [x] | `nava-lexer` unit tests | Phase 1 |
| Diagnostics with line/column | [x] | parser + driver tests | caret rendering included |
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
| `number`, `string`, `boolean`, `void` | [ ] | — | Phase 2 |
| Arrays | [ ] | — | Phase 2 |
| Object types | [ ] | — | Phase 2 |
| Function signatures | [ ] | — | Phase 2 |
| Type inference | [ ] | — | Phase 2 |
| Assignability checks | [ ] | — | Phase 2 |
| Structural typing | [ ] | — | Phase 6 |
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
| `console.log` | [ ] | — | Phase 5 |
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
| Arrow functions (`=>`) | lexed as `=` + `>`; not parsed | Phase 6 |
| Hex / binary / octal / bigint literals | lexer accepts decimal digits only | Phase 6 |
| Compound operators (`+=`, `++`, `**`) and optional chaining (`?.`) | lexed as separate tokens, not parsed | Phase 6 |
| Non-ASCII identifiers | lexer accepts ASCII letters, `_`, `$` only | Phase 6 |
| `\u` / `\x` string escapes | only `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\'` decoded | Phase 6 |
