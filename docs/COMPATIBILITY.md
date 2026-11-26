# NavaScript — TypeScript Compatibility Matrix

This matrix is the authoritative record of what NavaScript supports relative to
TypeScript. A feature is only marked supported when a test proves it.

**Legend:** `[x]` supported + tested · `[~]` partial · `[ ]` not started ·
`[-]` intentionally unsupported / divergent (with note)

Last updated: 2026-11-25 — Phases 0–8 and Phase 8.1 are complete. Phase 7
measured the matrix below against `tsc` + Node; Phase 8 measured speedups;
Phase 8.1 added tagged `any`/unions, conservative GC and per-file diagnostics.
Syntax, type-system, IR, codegen and native-execution entries are implemented
and tested. The runtime provides strings, arrays, objects, closure environments
and fatal error reporting; Phase 6 has delivered loops with
`break`/`continue`, the additional literals/operators (compound assignment,
`++`/`--`, `**`, ternary, hex/binary/octal, Unicode identifiers and `\x`/`\u`
escapes), arrow functions, optional member/index chaining, bigint literals and
optional calls, generics, classes, unions/intersections, full structural
typing, modules, closures/higher-order functions (ADR-0027) and same-function
exceptions (ADR-0028) and synchronous async/await (ADR-0029). All Phase 6
expansion items are delivered.

## 1. Syntax

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| Lexical keywords & tokens | [x] | `tests/lexer/` | Phase 1 |
| Variables (`let`/`const`/`var`) | [x] | `tests/parser/` | Phase 1 |
| Functions & returns | [x] | `tests/parser/` | Phase 1 |
| Expressions & operator precedence | [x] | `tests/parser/` | Phase 1 |
| Blocks & lexical scope | [x] | `tests/parser/` + `tests/typechecker/` | AST blocks parsed; scopes and shadowing checked in Phase 2 |
| `if`/`else` | [x] | `tests/parser/` | Phase 1 |
| Loops (`while`, `do-while`, `for`, `for-of`) | [x] | `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; `for-of` over arrays/strings (both forms); `break`/`continue` supported |
| Compound operators (`+= -= *= /= %=`, `++`, `--`, `**`) | [x] | `tests/parser/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; string `+=` concatenates |
| Ternary `?:` | [x] | `tests/parser/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6 |
| Hex/binary/octal literals & `_` separators | [x] | `tests/lexer/`, `tests/parser/` | Phase 6 |
| Bigint literals (`123n`) | [x] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs` | Phase 6; radix/separator forms decode to a signed 64-bit value; out-of-range and fractional suffixes are diagnosed; i64 arithmetic/comparisons and `console.log` work natively |
| Non-ASCII identifiers | [x] | `tests/lexer/` | Phase 6 |
| `\x`/`\u` string escapes | [x] | `tests/lexer/`, `tests/parser/` | Phase 6 (`\xNN`, `\uNNNN`, `\u{...}`) |
| Optional chaining (`?.`) | [~] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; `a?.b`/`a?.[i]` with reference-typed results; IR emits an `IsNull` branch, but a nullable receiver (`T | null`) is diagnosed by the checker, so the null branch is not reachable through `?.` yet (ADR-0025); scalar results are diagnosed and `undefined` does not exist |
| Exceptions (`throw`, `try`/`catch`/`finally`) | [~] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; same-function handling with ordinary IR branches, nested try, optional `finally`; catch bindings need a type annotation; cross-function throws, abrupt control flow out of protected regions and throw in catch/finally are diagnosed (ADR-0028) |
| Optional call (`a?.()`) | [~] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs` | Phase 6; type-checked for callability/arity; named callees lower as direct calls and function values as indirect calls (ADR-0027); no null branch because NavaScript has no nullable callable values |
| Arrow functions | [~] | `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; any expression position, annotated parameters required, shared captures and first-class values (ADR-0027); unannotated shorthand arrows are diagnosed |
| Function type annotations (`(n: T) => R`) | [x] | `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; resolved to `Type::Function`/`IrType::Function`; usable in variables, parameters, returns, arrays and object fields |
| Closures / function values | [x] | `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs` | Phase 6; captured bindings are shared cells, nested closures and named function values work; see `examples/closures.ns` (ADR-0027) |
| Comments | [x] | `tests/lexer/` | Phase 1 |
| Diagnostics with line/column | [x] | `tests/parser/` + `tests/driver/` | caret rendering included |
| Classes | [x] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs` | Phase 6; fields, constructors, methods, `this`, `super(...)`, `extends`, inherited members/constructors; static dispatch only, no field initialisers (ADR-0024) |
| Generics | [x] | `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs` | Phase 6; generic functions and interfaces with inferred/explicit type arguments, monomorphized lowering; generic type aliases and constraints out of scope |
| Interfaces | [x] | `tests/parser/` + `tests/typechecker/` | parsed and type-checked in Phase 2; full structural typing in Phase 6 with optional properties and intersection merging |
| Optional properties (`field?: T`) | [x] | `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; resolve to `T | null`, may be omitted from object literals; omitted reference fields are null-initialized |
| Type aliases | [x] | `tests/parser/` + `tests/typechecker/` | chains and cycles handled in Phase 2 |
| Unions / intersections | [~] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs`, `examples/tagged_values.ns` | nullable reference unions and intersections as before; mixed-representation unions now use tagged values with explicit boxing (ADR-0034) |
| Modules (`import`/`export`) | [~] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/driver/native.rs` | Phase 6; import-graph resolution from the entry file with validated exports; one module-wide namespace, relative `.ns` paths only; diagnostics rendered against combined source (ADR-0026) |
| Async/await, promises | [~] | `tests/lexer/`, `tests/parser/`, `tests/typechecker/`, `tests/ir/`, `tests/codegen/`, `tests/driver/native.rs` | Phase 6; `async function` returns a `Promise<T>` object and `await` unwraps it synchronously; no suspension, event loop, combinators or `.then`/`.catch` (ADR-0029) |
| Decorators | [-] | — | Deferred; not in initial scope |
| JSX (NavaScript `.nsx`, TypeScript `.tsx`) | [-] | — | Deferred |

## 2. Type system

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| `number`, `string`, `boolean`, `void` | [x] | `tests/typechecker/` | Phase 2 |
| Class instances | [x] | `tests/typechecker/`, `tests/driver/native.rs` | Phase 6; nominal `ClassInstance` with base-first field layout; assignability is structural over fields (full nominal subtyping is pending) |
| `bigint` | [~] | `tests/typechecker/`, `tests/driver/native.rs` | Phase 6; signed 64-bit integer; arithmetic/comparisons require two bigints (mixing with `number` is diagnosed); `+ - *` wrap on overflow, div/mod/pow are checked; arbitrary precision out of scope |
| `null` | [~] | manual check | type-checks; `console.log(null)` prints `null`, but an inferred `null` value cannot be lowered (opaque type) |
| Arrays | [x] | `tests/typechecker/` | `T[]` and array literals |
| Object types | [x] | `tests/typechecker/` | object literals and member access |
| Function signatures | [x] | `tests/typechecker/` | `(a: T) => R`, arity and argument checks |
| Type inference | [x] | `tests/typechecker/` | variables and return types |
| Assignability checks | [x] | `tests/typechecker/` | assignments, arguments, returns |
| Structural typing | [x] | `tests/typechecker/`, `tests/ir/`, `tests/driver/native.rs` | Phase 6; union/intersection-aware assignability, optional properties, merged intersection layouts, class instances |
| Generics | [x] | `tests/typechecker/`, `tests/ir/` | Phase 6; generic parameters unify with argument types; annotated generic references (`Box<number>`) resolve to concrete object types; bad references are diagnosed |
| `Promise<T>` | [~] | `tests/typechecker/`, `tests/ir/`, `tests/codegen/` | Phase 6; built-in generic resolving to a one-slot object (`Promise<void>` is `void`); values come from `async function`, not `Promise.resolve` (ADR-0029) |
| `any` | [~] | `examples/tagged_values.ns`, `tests/typechecker/checker.rs` | tagged runtime value; explicit `to_any_*` / `as_*` conversions; no automatic coercion or narrowing (ADR-0034) |
| `unknown` | [ ] | — | internal inference fallback only |
| Conditional / mapped types | [-] | — | Long-term |

## 3. Runtime behavior

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| Arithmetic / comparisons | [x] | `tests/driver/native.rs`, `tests/codegen/` | native `fadd/fsub/fmul/fdiv/frem`, `fcmp`/`icmp`; boolean results print correctly |
| Recursion | [x] | `tests/driver/native.rs` | fibonacci calls itself natively and prints `55` |
| Strings | [x] | `tests/driver/native.rs` | literal, `+`, `length`, indexing, `==`, lexicographic `< <= > >=`, `console.log`, `str_slice`, `str_index_of`, `str_char_code_at`, `str_compare` (ADR-0033) |
| Arrays & objects | [x] | `tests/driver/native.rs` | literals, get/set, `length`, runtime bounds checks |
| Closures / function values | [x] | `tests/driver/native.rs`, `examples/closures.ns` | Phase 6; shared captured bindings, higher-order calls and nested closures run natively (ADR-0027) |
| Async/await | [~] | `tests/driver/native.rs`, `examples/async_await.ns` | Phase 6; `async function` and `await` run natively with synchronous promise values (ADR-0029) |
| Exceptions | [~] | `tests/driver/native.rs`, `examples/exceptions.ns` | Phase 6; same-function `throw`/`try`/`catch`/`finally` run natively without a runtime ABI (ADR-0028); runtime bounds/field errors are still fatal |
| `console.log` | [~] | `tests/driver/native.rs` | numbers, booleans, strings and bigints print natively; object/array values are not meaningfully supported (raw bytes printed); no fix scheduled in the current Phase 6 increment |
| Memory management | [~] | `examples/gc.ns`, `tests/driver/native.rs` | conservative mark-sweep via `collect()` with block headers and root scanning; explicit, may retain garbage (ADR-0034) |
| File I/O and process arguments | [x] | `examples/self_host_stdlib.ns`, `tests/driver/native.rs` | typed `read_file`, `write_file`, `get_argv` builtins (ADR-0033) |
| Array growth helpers | [x] | `examples/self_host_stdlib.ns` | `array_push_number`, `array_push_string` return a new array |

## 4. Platform

| Feature | Status | Tests | Notes |
| --- | --- | --- | --- |
| macOS ARM64 native executable | [x] | `tests/driver/native.rs` | host triple detected via llvm-config |
| x86-64 macOS | [ ] | — | LLVM target available; not yet validated |
| Linux / Windows | [-] | — | Phase 11 |
| Debug info / debugger | [ ] | — | Phase 11 |
| No Node.js dependency at runtime | [x] | `tests/driver/native.rs` | native Mach-O, C runtime shim only |

## 5. Compatibility testing strategy (Phase 7 — implemented)

- The corpus is `tests/compatibility/corpus/<id>/`: a `case.json`, a TypeScript
  program and its NavaScript counterpart (multi-file cases add more sources).
- `scripts/compat.py` runs each case through `tsc` + Node and through the
  NavaScript CLI, comparing normalised stdout and exit status.
- Outcomes are pass / fail / skipped / blocked; `expect` is `match`, `differ`
  (documented divergence) or `skip` (declared unsupported). Differ cases pin
  the measured Node output and the expected NavaScript outcome.
- The committed report is `docs/COMPATIBILITY_REPORT.md`: 32 cases, 19 executed
  (12 `match`, 7 `differ`), 13 skips, 0 failures. Regenerate it with
  `python3 scripts/compat.py`; CI verifies it with `--check`.
- Re-run on every milestone touching semantics; regressions must be investigated.
- The reference is the runtime behaviour of tsc-emitted JavaScript. tsc's own
  exit status is not a case failure when JavaScript was emitted; the `type_error`
  case records this. Skipped cases are declared from §6 and are not executed.

## 6. Unsupported features register

Maintain this list so gaps are explicit and never silently miscompiled. Phase 7
adds a representative `tests/compatibility/corpus/<id>/` case for the major rows;
the measured outcomes are linked from `docs/COMPATIBILITY_REPORT.md`:

| Feature | Reason | Planned phase |
| --- | --- | --- |
| Decorators | scope control | deferred |
| JSX / TSX | no DOM/UI target | deferred |
| Conditional / mapped types | high complexity | long-term |
| Full `lib.d.ts` | large surface | long-term |
| Node.js interop beyond documented subset | platform scope | Phase 7 best effort |
| `eval` / dynamic code | incompatible with native model | not planned |
| `strictFunctionTypes` parameter contravariance | function parameters are checked covariantly | deferred |
| Mixed string/number `+` coercion | supported in Phase 5 via runtime number-to-string conversion | Phase 7 (full JS coercion rules) |
| Indirect call signatures | calls through function values cast the code pointer to the statically known signature; a mismatched function type is diagnosed by the checker, not converted at runtime | Phase 6 (documented) |
| Garbage collection | conservative mark-sweep is explicit (`collect()`), not automatic or generational (ADR-0034) | incremental/automatic collection |
| String concatenation with objects/functions | diagnosed instead of lowering a pointer through number-to-string coercion | documented |
| Per-file name scoping | diagnostics are per-file but imports still share one namespace (ADR-0034) | future phase |
| Automatic coercion for `any` / unions | values must be explicitly boxed and unboxed; no narrowing from tag tests | future phase |
| Per-file module namespaces | imported files are concatenated into one program, so names are module-wide | deferred (ADR-0026) |
| `.d.ts` and package resolution | only relative `.ns`/extensionless paths are resolved from the entry file | Phase 7 |
| Per-module diagnostic attribution | line/column refer to the concatenated source, not the originating file | deferred (ADR-0026) |
| Mixed-representation unions | now boxed into tagged values; untagged assignment is diagnosed; nullable primitives still rejected | Phase 8.1 (tagged values) |
| Optional chaining on nullable receivers | IR emits `IsNull`, but the checker rejects `T | null` receivers; narrowing on `b != null` works | deferred |
| Tagged unions and discriminated narrowing | narrowing covers null tests in `if`/`while`/`&&`/`||` only, not ternaries or assignments | deferred |
| Optional non-reference field defaults | omitted optional numeric/bigint/boolean fields are zero-initialized rather than `undefined` | Phase 6 (documented) |
| Virtual method dispatch | class methods are mangled direct calls; a base-typed reference calls the base implementation | deferred (ADR-0024) |
| Class field initialisers | fields are declared without initialisers; assign them in a constructor | deferred (ADR-0024) |
| Recursion via a captured arrow binding | a `const`/`let` arrow binding is not initialised when the arrow is created, so self-capture is unsupported; use a named function | Phase 6 (documented) |
| Nullable function values | optional calls have no null branch because a callable value cannot be null; `console.log` of a function value is diagnosed | Phase 6 (documented) |
| Nested function types in annotations | `() => (() => number)` hits a parser ambiguity; introduce a type alias (`type F = () => number`) instead | Phase 6 (documented) |
| Cross-function exception unwinding | a `throw` must have a handler in the same function; throwing inside a called function or an arrow is diagnosed (ADR-0028) | deferred |
| Abrupt control flow in protected regions | `return`/`break`/`continue` inside try/catch/finally and `throw` inside catch/finally are diagnosed; `try` needs a catch clause | deferred |
| Catch bindings | a type annotation is required and constrains throw types, unlike JavaScript's untyped `catch (e)` | deferred |
| Runtime errors | bounds/field/runtime failures still terminate the process; they are not catchable exceptions | deferred |
| `for-of` over general iterables | only arrays and strings are lowered (index-based); objects and custom iterables are unsupported | deferred |
| Asynchronous suspension | promises are synchronous values; there is no event loop, microtask queue or suspension at `await` (ADR-0029) | deferred |
| Promise combinators and methods | `.then`, `.catch`, `Promise.resolve` and `Promise.all` are not provided; promises come from `async function` only | deferred |
| Async arrows and methods | only `async function` declarations are parsed; `async` arrows/methods are diagnosed | deferred |
| `await` context | top-level and async-function `await` are supported; `await` in a non-async function is diagnosed | Phase 6 (documented) |
