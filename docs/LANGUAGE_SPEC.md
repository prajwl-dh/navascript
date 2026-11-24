# NavaScript — Language Specification (living document)

NavaScript uses TypeScript syntax. This document describes the **supported subset**
and the intent to grow toward broad compatibility. Status values mirror
`docs/COMPATIBILITY.md`: `[x]` supported/tested, `[~]` partial, `[ ]` planned.

> Full TypeScript compatibility is a long-term target. Never claim 100%
> compatibility until verified by the compatibility suite.

## 1. Source files

- `.ns` is the source extension for NavaScript programs. A JSX-like `.nsx`
   extension is planned for a later phase.
- One module per file; the driver resolves the import graph from the entry file
  and compiles all modules as one program (Phase 6, ADR-0026). Names share a
  module-wide namespace.
- Encoding: UTF-8.

## 2. Lexical structure

- [x] Keywords: `let`, `const`, `var`, `function`, `return`, `if`, `else`,
  `true`, `false`, `null`, `void`, `type`, `interface`, `while`, `do`, `for`,
  `break`, `continue`.
- [ ] Keywords `switch`, `async`, `await`, `throw`,
  `try`, `catch`, `finally` (their features are unimplemented; not yet
  recognized by the lexer). `of` in `for-of` is contextual, not a keyword.
- [x] Identifiers: Unicode letters, `_`, `$`, then alphanumeric/`_`/`$`.
- [x] Numeric literals: decimal integers and floats (including exponents),
  hex (`0x`), binary (`0b`) and octal (`0o`) integers, and `_` separators.
- [x] Bigint literals (`123n`, `0xffn`, `1_000n`) — Phase 6; decoded to a signed
  64-bit value. Out-of-range literals and a `n` suffix on a fractional/exponent
  form are diagnostics; a suffix that starts a longer identifier (`123nfoo`) is
  not treated as a bigint.
- [x] String literals: single and double quotes with `\n`, `\t`, `\r`, `\0`,
  `\\`, `\"`, `\'`, `\xNN`, `\uNNNN` and `\u{...}` escapes. Template literals
  are not yet supported.
- [x] Operators: arithmetic (`+ - * / % **`), comparison (`== === != !== < <=
  >= >`), logical (`&& ||`), unary (`! - + ++ --`), assignment (`=`), compound
  assignment (`+= -= *= /= %=`), ternary (`?:`), and `=>` in function *type
  annotations*.
- [x] Optional chaining (`a?.b`, `a?.[i]`) — Phase 6; reference-typed results
  only. The null branch is currently unobservable because no null object/array
  can be constructed; the result is typed as `T` until union types land.
- [x] Optional calls (`a?.()`) — Phase 6; parsed and type-checked for callability
  and arity. Named callees lower as direct calls; function values lower as
  indirect calls (ADR-0027). No null branch is emitted because NavaScript has no
  nullable callable values (ADR-0020).
- [x] Comments: `//` and `/* */`.
- [x] Every token carries byte offsets; diagnostics derive 1-based line and
  column from them.

## 3. Types

### Primitives (Phase 2, native values in Phase 4)
- [x] `number` — 64-bit floating point (ADR-0006); emitted as LLVM `double`.
- [~] `bigint` — Phase 6; signed 64-bit integer emitted as LLVM `i64`.
  Arithmetic requires two bigints (mixing with `number` is diagnosed);
  `+ - *` wrap on overflow while div/mod/pow are checked. Arbitrary precision
  is out of scope.
- [x] `boolean` — emitted as LLVM `i1`; prints `true`/`false`.
- [x] `string` — NUL-terminated arena buffers (Phase 5); literals, `+`,
  `length`, indexing and equality work natively.
- [x] `void`
- [~] `null` — type-checks and prints as `null`; usable as the null member of a
  nullable reference union (`string | null`), where narrowing and `?.` observe
  it. A bare inferred `null` still cannot be lowered (its IR type is opaque).
- [ ] `undefined` — not yet implemented.

### Composites (Phase 2 type checking; native layout in Phase 5)
- [x] Arrays `T[]` — type-checked; native literals, indexing, assignment and
  `length` work (Phase 5).
- [x] Object types `{ x: number }` — type-checked with structural field checks;
  native field layout works (Phase 5).
- [x] Function signatures `(a: number) => string` — type-checked; arity and
  argument types checked at calls. Function types are first-class values
  (ADR-0027): they may be stored in variables, arrays and object fields,
  passed as arguments and returned; `console.log` of a function value is
  diagnosed. A nested function type in an annotation needs a type alias
  because of a parser ambiguity.
- [x] Type aliases `type Foo = ...`, including chains and cycle diagnostics.
- [x] Interfaces `interface Foo { ... }`, with structural field checking.

### Advanced (Phase 6+)
- [x] Generics (Phase 6) — generic functions (`function id<T>(x: T): T`) and
  interfaces (`interface Box<T> { value: T; }`). Type arguments are inferred
  from call arguments or written explicitly (`id<number>(1)`); each distinct
  instantiation is lowered to a monomorphized native function (ADR-0023).
  Generic type aliases and constraints are not supported.
- [x] Classes and inheritance (Phase 6) — `class C extends B` with fields,
  constructors, methods, `new C(...)`, `this` and `super(...)`. Inherited fields
  use a stable base-first layout and inherited constructors/methods are
  available (ADR-0024). Dispatch is static (no virtual methods), and field
  initialisers are not supported — assign fields in a constructor.
- [x] Structural typing (Phase 6) — union/intersection-aware assignability,
   optional properties, and merged layouts for interfaces and class instances.
- [x] Union / intersection types (Phase 6) — `A | B` and `A & B` with nullable
   reference unions (`T | null`), control-flow narrowing on null tests in
   `if`/`while`/`&&`/`||`, and merged object layouts (ADR-0025). Mixed
   representations such as `number | string` are diagnosed.
- [x] Optional properties (`field?: T`) (Phase 6) — may be omitted from object
   literals; omitted reference fields are initialized to null and scalar ones
   to zero. Optional *parameters* remain unsupported.
- [ ] Overloads
- [ ] `any` / `unknown` (restricted semantics; document divergence)
- [ ] Decorators (deferred)
- [ ] Conditional / mapped types (long-term)

## 4. Expressions

- [x] Literals (number, bigint, string, boolean, null), identifiers
- [x] Binary/unary operators with TypeScript precedence
- [x] Function calls (direct calls lower and link natively)
- [x] Member access `obj.prop` (parsed, type-checked, lowered; native field
  layout works — Phase 5)
- [x] Array indexing `arr[i]` (parsed, type-checked; native indexing works — Phase 5)
- [x] Optional member/index access `a?.b` / `a?.[i]` (Phase 6; reference-typed
  results only)
- [x] Optional calls `a?.()` (Phase 6; direct or indirect depending on the
  callee — see above)
- [x] `new C(...)`, `this` and `super(...)` (Phase 6; inside class members)
- [x] Object literals (parsed, type-checked; native layout works — Phase 5)
- [x] Arrow functions (Phase 6) — `x => expr`, `(a, b) => expr`,
  `(a: T): R => { ... }`; any expression position, with annotated parameters.
  Unannotated shorthand arrows are diagnosed by the checker.
- [x] Closures / captured environments (Phase 6; ADR-0027) — captured bindings
  are shared cells, so closures observe assignments in the enclosing frame and
  vice versa; nested closures share the same cell. `this` is captured by value.
- [ ] Template literals (Phase 6)
- [x] Ternary `cond ? a : b` (Phase 6)
- [x] Exponentiation `a ** b` (right-associative, Phase 6)
- [x] Compound assignment (`+=`, `-=`, `*=`, `/=`, `%=`) and `++`/`--`
  (prefix and postfix, Phase 6)

## 5. Statements

- [x] Variable declarations (`let`, `const`, `var`)
- [x] Expression statements
- [x] Blocks and lexical scope (checked by the type checker)
- [x] `if` / `else`, including unbraced single-statement branches
- [x] `return`
- [x] `while`, `do-while`, C-style `for` (`init; condition; update`) and
  `for-of` over arrays and strings (Phase 6). Both `for (let x of xs)` and
  `for (x of xs)` are accepted. `break` and `continue` exit/skip the innermost
  loop; `continue` in a C-style `for` runs the update expression. Iteration over
  other iterables is not implemented.
- [ ] `throw` / `try` / `catch` / `finally` (Phase 6; ADR-0015)
- [ ] `switch`

## 6. Functions

- [x] Named function declarations (emitted as native functions)
- [x] Parameters and return type annotations
- [x] Inference for return types
- [x] Generic functions with inferred or explicit type arguments (Phase 6)
- [x] Recursion — the fibonacci acceptance program runs natively
- [x] Nested functions / closures (Phase 6; ADR-0027) — arrows capture shared
  bindings from enclosing functions; `examples/closures.ns` runs natively.
- [x] Higher-order functions (Phase 6; ADR-0027) — function values pass through
  parameters, returns, arrays and object fields and are called via an indirect
  ABI. Recursive self-capture of an arrow binding is unsupported; use a named
  function.
- [ ] Async functions and promises (Phase 6)

## 7. Modules

- [x] `import { name } from "./path"` and `export <declaration>` (Phase 6).
   `from` is contextual, and imported names must be exported.
- [~] Module resolution rules compatible with TypeScript where practical —
   Phase 6 resolves relative paths and extensionless `.ns` specifiers from the
   entry file, deduplicates visited files and handles cycles; `.d.ts`, package
   resolution and per-file namespaces are not implemented (ADR-0026).
- [ ] JavaScript interop (Phase 7, best effort)

## 8. Runtime semantics

- [x] Memory allocation: program-lifetime bump arena (ADR-0005); no
  reclamation during execution (documented divergence from JS GC).
- [x] Strings: NUL-terminated arena buffers; literals, concatenation, `length`,
  indexing and equality work natively.
- [x] Arrays and objects at runtime: typed slot accessors with bounds and field
  validation; fatal diagnostics on invalid access.
- [x] Closures (Phase 6, ADR-0027) — function values are runtime objects with
  a code pointer and captured bindings; closures receive the object as a hidden
  parameter and use the typed object slot accessors. Allocations live in the
  program-lifetime arena.
- [ ] Exceptions — `throw`/`try` are unparsed; runtime errors currently
  terminate the process with a diagnostic (Phase 6, ADR-0015).
- [~] `console.log` — numbers, booleans, strings and bigints print natively;
  object and array values are not meaningfully supported (their raw bytes are
  printed). No fix is scheduled in the current Phase 6 increment.

## 9. First acceptance program

```typescript
function fibonacci(n: number): number {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log(fibonacci(10)); // prints 55
```

Required behavior: read `.ns` → parse → type-check → NavaScript IR → LLVM IR →
native macOS executable → run → print `55`. No Node.js or JavaScript runtime.

**Status (Phase 4):** achieved and covered by `tests/driver/native.rs`. The
same fixture also parses (Phase 1), type-checks (Phase 2) and lowers to verified
IR (Phase 3).

## 10. Explicit non-goals (initial phases)

- DOM/browser APIs.
- Node.js module ecosystem compatibility beyond documented interop.
- Full `lib.d.ts`.
- JSX/TSX rendering.
- Dynamic `eval`.
