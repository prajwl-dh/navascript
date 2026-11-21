# Nava — Language Specification (living document)

Nava uses TypeScript syntax. This document describes the **supported subset**
and the intent to grow toward broad compatibility. Status values mirror
`docs/COMPATIBILITY.md`: `[x]` supported/tested, `[~]` partial, `[ ]` planned.

> Full TypeScript compatibility is a long-term target. Never claim 100%
> compatibility until verified by the compatibility suite.

## 1. Source files

- `.ts` is the initial source extension. `.tsx` is planned for a later phase.
- One module per file initially; module resolution comes in Phase 6/7.
- Encoding: UTF-8.

## 2. Lexical structure

- [x] Keywords: `let`, `const`, `var`, `function`, `return`, `if`, `else`,
  `true`, `false`, `null`, `void`, `type`, `interface`, `while`, `do`, `for`,
  `break`, `continue`.
- [ ] Keywords `switch`, `class`, `import`, `export`, `async`, `await`, `throw`,
  `try`, `catch`, `finally` (their features are unimplemented; not yet
  recognized by the lexer). `of` in `for-of` is contextual, not a keyword.
- [x] Identifiers: Unicode letters, `_`, `$`, then alphanumeric/`_`/`$`.
- [x] Numeric literals: decimal integers and floats (including exponents),
  hex (`0x`), binary (`0b`) and octal (`0o`) integers, and `_` separators.
  Bigint (`123n`) is not yet supported.
- [x] String literals: single and double quotes with `\n`, `\t`, `\r`, `\0`,
  `\\`, `\"`, `\'`, `\xNN`, `\uNNNN` and `\u{...}` escapes. Template literals
  are not yet supported.
- [x] Operators: arithmetic (`+ - * / % **`), comparison (`== === != !== < <=
  >= >`), logical (`&& ||`), unary (`! - + ++ --`), assignment (`=`), compound
  assignment (`+= -= *= /= %=`), ternary (`?:`), and `=>` in function *type
  annotations*.
- [x] Optional chaining (`a?.b`, `a?.[i]`) — Phase 6; reference-typed results
  only. No optional call `a?.()` yet, and the null branch is currently
  unobservable because no null object/array can be constructed; the result is
  typed as `T` until union types land.
- [ ] Bigint literals.
- [x] Comments: `//` and `/* */`.
- [x] Every token carries byte offsets; diagnostics derive 1-based line and
  column from them.

## 3. Types

### Primitives (Phase 2, native values in Phase 4)
- [x] `number` — 64-bit floating point (ADR-0006); emitted as LLVM `double`.
- [x] `boolean` — emitted as LLVM `i1`; prints `true`/`false`.
- [x] `string` — NUL-terminated arena buffers (Phase 5); literals, `+`,
  `length`, indexing and equality work natively.
- [x] `void`
- [~] `null` — type-checks; `console.log(null)` prints `null`, but an inferred
  `null` value cannot be lowered (its IR type is opaque). Full semantics TBD.
- [ ] `undefined` — not yet implemented.

### Composites (Phase 2 type checking; native layout in Phase 5)
- [x] Arrays `T[]` — type-checked; native literals, indexing, assignment and
  `length` work (Phase 5).
- [x] Object types `{ x: number }` — type-checked with structural field checks;
  native field layout works (Phase 5).
- [x] Function signatures `(a: number) => string` — type-checked; arity and
  argument types checked at calls.
- [x] Type aliases `type Foo = ...`, including chains and cycle diagnostics.
- [x] Interfaces `interface Foo { ... }`, with structural field checking.

### Advanced (Phase 6+)
- [ ] Generics
- [ ] Classes and inheritance
- [ ] Structural typing
- [ ] Union / intersection types
- [ ] Optional properties and parameters
- [ ] Overloads
- [ ] `any` / `unknown` (restricted semantics; document divergence)
- [ ] Decorators (deferred)
- [ ] Conditional / mapped types (long-term)

## 4. Expressions

- [x] Literals (number, string, boolean, null), identifiers
- [x] Binary/unary operators with TypeScript precedence
- [x] Function calls (direct calls lower and link natively)
- [x] Member access `obj.prop` (parsed, type-checked, lowered; native field
  layout works — Phase 5)
- [x] Array indexing `arr[i]` (parsed, type-checked; native indexing works — Phase 5)
- [x] Optional member/index access `a?.b` / `a?.[i]` (Phase 6; reference-typed
  results only)
- [x] Object literals (parsed, type-checked; native layout works — Phase 5)
- [x] Arrow functions (Phase 6) — `x => expr`, `(a, b) => expr`,
  `(a: T): R => { ... }`; supported as variable initialisers with annotated
  parameters and no captured locals
- [ ] Closures / captured environments (Phase 6; ADR-0015)
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
- [x] Recursion — the fibonacci acceptance program runs natively
- [ ] Nested functions / closures (Phase 6; ADR-0015)
- [ ] Higher-order functions (Phase 6)
- [ ] Async functions and promises (Phase 6)

## 7. Modules

- [ ] `import` / `export` syntax (Phase 6/7)
- [ ] Module resolution rules compatible with TypeScript where practical
- [ ] JavaScript interop (Phase 7, best effort)

## 8. Runtime semantics

- [x] Memory allocation: program-lifetime bump arena (ADR-0005); no
  reclamation during execution (documented divergence from JS GC).
- [x] Strings: NUL-terminated arena buffers; literals, concatenation, `length`,
  indexing and equality work natively.
- [x] Arrays and objects at runtime: typed slot accessors with bounds and field
  validation; fatal diagnostics on invalid access.
- [ ] Closures (Phase 6, ADR-0015)
- [ ] Exceptions — `throw`/`try` are unparsed; runtime errors currently
  terminate the process with a diagnostic (Phase 6, ADR-0015).
- [~] `console.log` — numbers, booleans and strings print natively; object and
  array values are not meaningfully supported (their raw bytes are printed).
  No fix is scheduled in the current Phase 6 increment.

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

Required behavior: read `.ts` → parse → type-check → Nava IR → LLVM IR →
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
