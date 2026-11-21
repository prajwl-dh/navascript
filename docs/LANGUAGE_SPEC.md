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

- [x] (planned Phase 1) Keywords: `let`, `const`, `var`, `function`, `return`,
  `if`, `else`, `for`, `while`, `type`, `interface`, `class`, `import`,
  `export`, `async`, `await`, and type keywords.
- [x] (planned Phase 1) Identifiers: Unicode-aware identifier start/continue
  (initial subset: ASCII letters, `_`, `$`, digits after first char).
- [x] (planned Phase 1) Numeric literals: decimal integers and floats initially;
  hex/binary/octal and separators later.
- [x] (planned Phase 1) String literals: double and single quotes; escaping;
  template literals later.
- [x] (planned Phase 1) Operators: arithmetic, comparison, logical, assignment,
  unary, ternary.
- [x] (planned Phase 1) Comments: `//` and `/* */`.
- [x] (planned Phase 1) Every token carries (line, column, byte offset).

## 3. Types

### Primitives (Phase 2)
- [ ] `number` — initially 64-bit floating point (JavaScript-like semantics).
  An integer mode may be introduced later via optimization evidence.
- [ ] `string` — immutable UTF-8/UTF-16 decision deferred to Phase 5.
- [ ] `boolean`
- [ ] `void`
- [ ] `null` / `undefined` semantics TBD (document explicitly before impl).

### Composites (Phase 2+)
- [ ] Arrays `T[]`
- [ ] Object types `{ x: number }`
- [ ] Function signatures `(a: number) => string`
- [ ] Type aliases `type Foo = ...`
- [ ] Interfaces `interface Foo { ... }`

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

- [ ] Literals, identifiers
- [ ] Binary/unary operators with TypeScript precedence
- [ ] Function calls
- [ ] Member access `obj.prop`
- [ ] Array indexing `arr[i]`
- [ ] Object literals
- [ ] Arrow functions / closures (Phase 5/6)
- [ ] Template literals (Phase 6)
- [ ] Ternary `cond ? a : b`

## 5. Statements

- [ ] Variable declarations (`let`, `const`, `var`)
- [ ] Expression statements
- [ ] Blocks and lexical scope
- [ ] `if` / `else`
- [ ] `return`
- [ ] `for`, `while`, `do-while`, `for-of`
- [ ] `throw` / `try` / `catch` / `finally` (Phase 5)
- [ ] `switch`

## 6. Functions

- [ ] Named function declarations
- [ ] Parameters and return type annotations
- [ ] Inference for return types
- [ ] Nested functions / closures
- [ ] Recursion (required for the fibonacci milestone)
- [ ] Higher-order functions (Phase 6)
- [ ] Async functions and promises (Phase 6)

## 7. Modules

- [ ] `import` / `export` syntax (parser Phase 1/6, resolution Phase 6/7)
- [ ] Module resolution rules compatible with TypeScript where practical
- [ ] JavaScript interop (Phase 7, best effort)

## 8. Runtime semantics (Phase 5)

- [ ] Memory allocation strategy documented in ADR-0005
- [ ] String representation and operations
- [ ] Arrays and objects
- [ ] Closures
- [ ] Exceptions and runtime error reporting
- [ ] `console.log` (required by fibonacci acceptance program)

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

Required behavior: read `.ts` → parse → type-check → LLVM IR → native macOS
executable → run → print `55`. No Node.js or JavaScript runtime.

## 10. Explicit non-goals (initial phases)

- DOM/browser APIs.
- Node.js module ecosystem compatibility beyond documented interop.
- Full `lib.d.ts`.
- JSX/TSX rendering.
- Dynamic `eval`.
