# NavaScript — Architecture Decision Records (ADRs)

Decisions are numbered and immutable once accepted. A superseding decision gets
a new number and references the old one. Keep entries short and factual.

---

## ADR-0001 — Project name: `navascript`

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The source spec used `nova` in some places and `navascript` in others.
- **Decision:** Use **navascript** as the project/language name. The CLI binary is
  `navascript`. Keep `nova0`/`nova1`/`nova2` as *bootstrap stage* names only, as
  defined by the bootstrap strategy, to avoid confusing the compiler name with
  stage artifacts.
- **Consequences:** All docs, paths, and commands use `navascript`; stage artifacts
  use the `novaN` convention from `BOOTSTRAP.md`.

## ADR-0002 — Rust as the bootstrap implementation language

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Need a productive language for initial compiler development;
  self-hosting is a later goal.
- **Decision:** Implement `nova0` in Rust; progressively rewrite in NavaScript
  (Phase 9) while keeping Rust functional.
- **Consequences:** Rust toolchain is a build-time dependency only; final NavaScript
  compilation must not need it.

## ADR-0003 — LLVM as the backend

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Need native ARM64 codegen with optimization; hand-writing
  assembly would be impractical.
- **Decision:** Use LLVM 23.1.6 for instruction selection, register allocation,
  optimization, and object emission; emit LLVM IR from NavaScript IR.
- **Consequences:** LLVM is confined to the `codegen` module. Front end stays
  backend-independent. Do not hardcode target arch; query the triple.

## ADR-0004 — LLVM integration method

- **Date:** 2026-11-20 (decision 2026-11-21)
- **Status:** Accepted
- **Context:** LLVM can be linked in-process via `llvm-sys` or driven as an
  external tool emitting textual IR. The environment has Homebrew LLVM 23.1.6
  with AArch64 and X86 targets, Apple Clang 21, but no `rustup`; `cmake` and
  `ninja` were absent at decision time (both were installed later on
  2026-11-21). Phase 3 already produces a platform-independent IR and a textual
  printer.
- **Options:**
  - A: `llvm-sys` crate with `LLVM_SYS_231_PREFIX=/opt/homebrew/opt/llvm`.
  - B: emit `.ll` and invoke `llc`/`clang` from the driver.
- **Decision:** **Option B.** The compiler emits textual LLVM IR, then invokes
  `llc` (from the detected LLVM prefix) to produce an object and `clang` to
  link it with a tiny C runtime. Reasons: no additional build tooling is
  required, the build stays reproducible on this machine, and the generated
  `.ll` is directly inspectable for debugging. The target triple is queried
  with `llvm-config --host-target`; it is never hardcoded to ARM64.
- **Consequences:** The `codegen` crate depends only on `navascript-ir` and the
  standard library — no LLVM Rust bindings. Build reliability now depends on
  the external toolchain being on `PATH`/at the LLVM prefix; the driver reports
  a clear error if `llc`, `llvm-config` or `clang` is missing.

## ADR-0005 — Memory management strategy

- **Date:** 2026-11-20 (decision 2026-11-21)
- **Status:** Accepted
- **Context:** Need a documented strategy for allocation and reclamation; a
  tracing GC is a large undertaking early on.
- **Decision:** Use a **program-lifetime bump arena** in the C runtime. Every
  runtime object (strings, arrays, objects) is allocated from one fixed arena,
  8-byte aligned, and is never individually freed. The arena is released when
  the process exits. Out-of-memory is a fatal runtime error with a clear
  message, not undefined behavior.
- **Consequences:** No reclamation happens during program execution, so long-
  running programs that allocate in loops will grow until the arena is
  exhausted. This is a deliberate, documented divergence from JavaScript's GC
  semantics. The compact ABI keeps the backend simple; a tracing collector or
  reference counting can replace `navascript_alloc` later without changing the IR or
  front end.

## ADR-0006 — Numeric model

- **Date:** 2026-11-20
- **Status:** Accepted (applied in Phases 2–4)
- **Context:** TypeScript `number` is IEEE-754 double; native code may benefit
  from integer representations.
- **Decision:** Use 64-bit floating point for JavaScript-like semantics. The
  type checker models `number` as `Type::Number`, IR as `IrType::Number`, and
  codegen as LLVM `double` with `fadd`/`fsub`/`fmul`/`fdiv`/`frem` and `fcmp`.
  Integer specialization would require benchmark evidence and explicit
  correctness rules.
- **Consequences:** Avoid silent semantic changes when optimizing.

## ADR-0007 — Keep three goals independent

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Compatibility, native compilation, and self-hosting are often
  conflated.
- **Decision:** Track and report them separately; never claim one as evidence
  of another.
- **Consequences:** `COMPATIBILITY.md`, `PROGRESS.md`, and `BOOTSTRAP.md` each
  measure their own goal.

---

## ADR-0008 — Phase 0 scope: verify LLVM, defer linking to Phase 4

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The originating specification lists "Integrate LLVM" in the
  Phase 0 foundation tasks *and* in the Phase 4 native-backend tasks. Linking
  LLVM during Phase 0 would pull backend work forward and break the
  front-end/back-end separation. A review of Phase 0 flagged the mismatch
  between that bullet and the recorded "Phase 0 complete" status.
- **Decision:** Phase 0 verifies the LLVM toolchain (version, targets, tools,
  prefix) and records the integration plan (ADR-0004). Actual LLVM linking,
  code generation and linking of executables belong to Phase 4. Phase 0's
  "complete" status refers to the foundation scope defined in `ROADMAP.md`,
  with this deviation explicitly documented rather than silently assumed.
- **Consequences:** The Phase 0 exit condition is a building CLI and test
  suite, not a native executable. `PROGRESS.md` and `ROADMAP.md` state the
  deviation; no claim of LLVM integration is made before Phase 4.

---

## ADR-0009 — Shared diagnostics leaf crate

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Phase 2 needs the type checker to add diagnostics in the same
  shape as the parser. Keeping `Diagnostic`/`Diagnostics` inside `navascript-parser`
  would force `navascript-typechecker` to depend on the parser for a data type, even
  though it only consumes the AST.
- **Decision:** Move `Diagnostic`, `Severity`, `LineIndex` and `Diagnostics`
  into a new dependency-light crate `navascript-diagnostics` with three real
  consumers (parser, type checker, driver). `navascript-parser` re-exports them, so
  the Phase 1 `parse` seam is unchanged.
- **Consequences:** One sorting/dedup/rendering implementation for every
  front-end phase; no port or sink trait until a second real adapter exists.

---

## ADR-0010 — Semantic analysis lives inside the type checker

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The 12-module architecture lists both "Semantic analyzer" and
  "Type checker" as separate modules. In practice their Phase 2 work — scope
  building, duplicate declarations, name resolution, unknown identifiers — is
  inseparable from symbol-table construction and inference. Splitting them
  would create a shallow middle module that passes data straight through.
- **Decision:** Implement these semantic checks inside `navascript-typechecker`
  behind its single `check` seam. `compiler/semantic/` remains reserved as a
  placeholder; if a genuinely independent semantic pass appears later, it can
  be extracted from the checker without changing the seam.
- **Consequences:** `navascript-semantic` has no implementation in Phase 2. This is a
  documented deviation, not an omission: the checks are implemented and tested
  in `navascript-typechecker`, satisfying "no feature claim without a passing test".

## ADR-0011 — Type-only `console` builtin

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** The fibonacci acceptance program calls `console.log`, but the
  runtime stdlib is Phase 5. Without a type for `console`, Phase 2 rejects the
  very program the project uses as its first milestone.
- **Decision:** Seed the checker with a type-only `console` symbol exposing
  `log(value: unknown): void`. No runtime implementation is implied.
- **Consequences:** `examples/fibonacci.ns` type-checks in Phase 2. The real
  `console.log` implementation and its full signature belong to Phase 5.

---

## ADR-0012 — All tests live in the top-level tests/ tree

- **Date:** 2026-11-20
- **Status:** Accepted
- **Context:** Phase 0–2 placed unit tests in `#[cfg(test)]` modules inside each
  crate and binary tests in `compiler/driver/tests/`, while the documented
  repository structure reserved a top-level `tests/` tree that stayed empty.
  That mismatch made the test layout hard to discover and drifted from the
  spec's structure. Cargo only auto-discovers `tests/` at a package root.
- **Decision:** Keep **all** tests under `tests/`, organised by module
  (`ast/`, `diagnostics/`, `lexer/`, `parser/`, `symbol/`, `typechecker/`,
  `driver/`), with each file importing only public interfaces. Register each
  file as an explicit `[[test]]` target in `compiler/driver/Cargo.toml`; add
  dev-dependencies there. No inline test modules remain in `compiler/`.
- **Consequences:** One discoverable test location matching the documented
  repository structure. Tests can reach only public interfaces, so interfaces
  become the test surface by construction. Every new module must add its test
  file under `tests/` and a corresponding `[[test]]` target. `HELP` was made
  public so its content could be tested from outside the driver crate.

---

## ADR-0013 — Stack slots instead of SSA phi nodes in NavaScript IR

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** The Phase 3 roadmap allows phi nodes or a documented alternative
  such as stack slots. LLVM's `mem2reg` pass reliably promotes simple stack
  slots to SSA, and our front end already allocates one location per variable.
  Phi insertion during lowering would complicate the lowering, the verifier
  and the planned NavaScript rewrite for no measurable benefit at this stage.
- **Decision:** NavaScript IR uses explicit `alloca`/`store`/`load` stack slots for
  local variables. Control flow is expressed only with unconditional and
  conditional branches; there are no phi nodes. Instruction results are
  SSA-like values but are scoped to their block's dominance region.
- **Consequences:** The verifier checks dominance of value definitions and
  stack-slot allocations. Phase 4 lowers slots to LLVM `alloca` and lets
  optimization promote them. If profiling later shows a need, SSA can be
  introduced as a separate IR pass without changing the front-end seam.

## ADR-0014 — The IR lowering seam

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 3 needs a platform-independent representation that Phase 4
  can lower to LLVM. The driver already owns the parsed program, the typed
  program and the source text.
- **Decision:** `navascript_ir::lower(&Program, &TypedProgram, &str) -> LowerOutput`
  is the single external seam. `LowerOutput` owns an `IrModule` plus
  `Diagnostics` in the same shape as earlier phases. `verify` and `print` are
  separate public functions because the verifier and IR debug output are
  explicit roadmap deliverables with real callers (driver, tests).
- **Consequences:** IR depends only on `navascript-ast`, `navascript-diagnostics` and
  `navascript-symbol` — never LLVM. The driver calls parse → check → lower and
  renders diagnostics from all three uniformly.

---

## ADR-0015 — Phase 5 scope: closures and exceptions deferred

- **Date:** 2026-11-21
- **Status:** Accepted (closures delivered by ADR-0027; exceptions by ADR-0028)
- **Context:** The Phase 5 roadmap lists closures/function values and
  exceptions alongside strings, arrays and objects. Closures require first-
  class function values, captured environments and an indirect-call ABI;
  JavaScript-style exceptions require unwinding support across generated
  frames. Neither exists in the IR or the LLVM backend yet.
- **Decision:** Implement strings, arrays, objects, arena allocation and fatal
  runtime error reporting in Phase 5. Defer closures and exceptions to Phase 6,
  recording them in `COMPATIBILITY.md`. Runtime bounds/field errors terminate
  the process with a diagnostic; this is a documented divergence from JS
  exceptions.
- **Consequences:** At the time no claim of exception support was made and
  `throw`/`try` were unparsed, so there was no silent miscompilation. ADR-0028
  later delivered same-function exceptions behind the existing `lower`/
  `generate` seams; cross-frame unwinding remains out of scope.

---

## ADR-0016 — Code splitting and removal of placeholder directories

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Several implementation files had grown past 600 lines
  (`parser/src/lib.rs`, `ir/src/lower.rs`, `codegen/src/lib.rs`,
  `typechecker/src/checker.rs`, `runtime/navascript_runtime.c`) and the repository
  carried empty `.gitkeep` directories for features not yet implemented.
  This made navigation harder and blurred the seams described in
  `DESIGN.md`.
- **Decision:** Split each large module into private implementation files
  behind its existing public seam (`parse`, `check`, `lower`, `generate`,
  `tokenize`). Split the C runtime into `runtime/{memory,strings,arrays,objects,
  console}/` sources plus a shared header, with `runtime/navascript_runtime.c`
  reduced to the process entry point. Remove empty placeholder directories
  (`bootstrap/`, `stdlib/`, `benchmarks/`, `runtime/exceptions/`,
  `tests/compatibility/`); recreate them when the corresponding phase begins.
- **Consequences:** All compiler public seams (`parse`, `check`, `lower`,
  `generate`) and every test target are unchanged; all 120 tests still pass.
  Two incidental public items of the runtime bundle changed:
  `navascript_codegen::RUNTIME_C` became `RUNTIME_FILES` (the runtime is now multiple
  sources), and `navascript_driver::build::emit_native` takes that source list. Both
  are build plumbing, not compiler interfaces, and their callers were updated
  in the same change. Files are now typically 100–300 lines. Each crate keeps
  one public façade and private internal modules, matching the deep-module
  philosophy. Most files are now 100–300 lines; two orchestration files
  (`ir/verify.rs`, `codegen/instructions.rs`) remain larger because splitting
  them further would obscure a single algorithm rather than clarify it.
  Future phases add directories only when they contain code.

---

## ADR-0017 — Phase 6 loop lowering

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 6 adds `while`, `do-while`, C-style `for` and `for-of`.
  TypeScript allows loops at program scope, and `for-of` has both a declaration
  (`let x of xs`) and an assignment (`x of xs`) form. The `of` token is widely
  used as an ordinary identifier, so making it a reserved keyword would break
  existing programs.
- **Decision:** Loops lower to basic-block back edges with no new instruction
  kinds; `for-of` over arrays/strings is desugared to an index stack slot plus
  `ArrayLength`/`StringLength` and `ArrayGet`/`StringCharAt`, so ADR-0013's slot
  model still applies. Program-scope loops become `ItemKind::Stmt` items lowered
  into the synthetic `main`; other statement kinds are not allowed at top level.
  `of` is recognised contextually in `for` headers only. Loop lowering lives in
  a private `ir/loops.rs` behind the existing `lower` seam.
- **Consequences:** No IR verifier or codegen changes were needed. `for-of` is
  limited to arrays and strings, as are arbitrary iterables. `while (true)` /
  `for (;;)` bodies that always return are treated as non-fallthrough rather
  than producing a spurious diagnostic. `break`/`continue` were later added in
  ADR-0021; this ADR's size guidance is still met.

---

## ADR-0018 — Phase 6 literals and operators

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 6's second syntax increment adds compound assignment,
  `++`/`--`, `**`, ternary, based integer literals with separators, Unicode
  identifiers and `\x`/`\u` escapes. Ternary can introduce basic blocks from
  inside any expression, while the existing `return` lowering assumed one
  block per statement.
- **Decision:** Ternary lowers to a result stack slot written in both arms and
  read in a join block (consistent with ADR-0013). `**` maps to the LLVM
  `@llvm.pow.f64` intrinsic, declared in codegen only. Radix-prefix knowledge
  lives in one shared `navascript_lexer::radix_prefix` helper used by both the
  scanner and parser. Invalid `\x`/`\u` escapes are lex diagnostics, never
  silently decoded to an empty string. The three files that grew past ADR-0016's
  size guidance were split into private modules behind their existing façades.
- **Consequences:** `return` and other statement lowering now use the current
  block after expression lowering, not the incoming block. At this point bigint,
  arrow functions and optional chaining were still unsupported (they were
  delivered in later Phase 6 increments; see ADR-0019, ADR-0020 and ADR-0022).
  No new runtime ABI functions were needed.

---

## ADR-0019 — Arrow functions as named initialisers

- **Date:** 2026-11-21
- **Status:** Superseded by ADR-0027 (the named-initialiser restriction and the
  "captured locals are diagnosed" rule no longer apply)
- **Context:** Phase 6 adds arrow expressions, but IR supports only direct
  named calls, and ADR-0015 defers captured environments and function values.
  Inferring an unannotated arrow parameter as `Unknown` let a wrong argument
  type compile successfully and produce a bogus value.
- **Decision:** Arrow initialisers lower to a named IR function named after the
  binding; calls resolve through that name. Parameters must have type
  annotations, checked before lowering. Captured locals and arrows used as
  values are diagnosed, never silently miscompiled. The checked function type's
  return type is scoped to arrow lowering rather than changing the general
  type-to-IR mapping. `parser/arrows.rs` and `typechecker/arrows.rs` keep the
  touched files within ADR-0016's size guidance.
- **Consequences:** At the time, first-class function values, higher-order
  functions and closures remained unsupported and were recorded in
  `COMPATIBILITY.md`. No new LLVM declarations or runtime ABI were needed.
  ADR-0027 supersedes the named-initialiser restriction: arrows now lower to
  closure values in any expression position.

---

## ADR-0020 — Optional chaining lowering

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 6 adds `a?.b` and `a?.[i]`. The existing `==` instruction
  has no pointer-comparison semantics, and reusing it for null checks would
  change comparison behaviour for every string expression.
- **Decision:** Introduce a dedicated `IsNull` IR instruction that codegen emits
  as `icmp eq ptr <value>, null`. Optional accesses allocate a result stack
  slot, branch to a null block (storing `null`) or a value block (performing the
  normal member/index access), then join. The checker allows only
  reference-typed results; scalar results and assignment through `?.` are
  diagnosed. The access descriptor is an enum-backed struct so member and
  index forms cannot be confused.
- **Consequences:** Existing `==`/`!=` semantics are untouched. Since no
  nullable object/array value can currently be constructed, the null branch is
  unobservable at the time, and results were typed `T` until unions landed in
  ADR-0025 (which made null tests and narrowing observable; a nullable `?.`
  receiver is still diagnosed).
  Optional calls `a?.()` were added in ADR-0022: the
  AST records the `optional` flag; named callees lower as direct calls and
  function values as indirect calls (ADR-0027). No null branch is emitted
  because callable values cannot be null. This is a documented limitation, not
  a silent miscompile; callability and arity are still checked, so `null?.()` is
  diagnosed. A nullable `?.` receiver is likewise diagnosed (see ADR-0025).

---

## ADR-0021 — Break and continue in loops

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Loops had no early-exit statements. `break` and `continue` need
  different targets (`continue` must reach the `for` update expression, the
  `do-while` condition block or the `while` header), and an always-abrupt loop
  body must not cause a targeted block to be removed.
- **Decision:** `FunctionBuilder` keeps a stack of `LoopContext` values with
  `break_target`, `continue_target` and a `has_jump` flag. `with_loop` pushes the
  context, lowers the body and pops it; `break`/`continue` branch to the chosen
  target and mark `has_jump`. Loop lowering consults `has_jump` before deleting
  otherwise-unreachable condition/update blocks. The type checker tracks
  `loop_depth` and diagnoses jumps outside loops; the IR repeats the check as a
  defence in depth.
- **Consequences:** Nested loops resolve to the innermost context. Loop jumps
  are ordinary `br` terminators, so no new LLVM instruction was needed. The
  extra `for_of.rs` module keeps `loops.rs` within ADR-0016's size guidance.

---

## ADR-0022 — Bigint and optional calls

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** The last syntactic rows of `COMPATIBILITY.md` §6 were bigint
  literals (`123n`) and optional calls (`a?.()`). The bigint row's own reason
  named three gaps: the suffix was not parsed, and there was no `bigint` type in
  the checker or runtime. A literal with no operations, however, is not testable
  as a feature, so the minimum useful implementation includes integer operators.
  Optional calls needed to coexist with the existing direct-call-only IR.
- **Decision:** The lexer recognises a word-final `n` suffix after an integer
  (radix prefixes and `_` separators included) as `TokenKind::BigInt`; a suffix
  on a fractional/exponent form is a lex diagnostic, and a suffix followed by an
  identifier character is not a bigint. `decode_bigint` parses through `i128`
  and diagnoses values outside `i64`. `bigint` is a checker primitive
  (`Type::BigInt`) lowering to LLVM `i64` (`IrType::BigInt`, `Value::BigInt`);
  bigint operators use a dedicated `Instruction::BigIntBinary` and
  `UnaryOp::NegBigInt`. `+ - *` use LLVM i64 semantics (wrap), while div, mod and
  power call checked runtime helpers (`runtime/bigint/bigint.c`) that diagnose
  zero divisors, negative exponents and overflowing powers. Bigint arrays and
  objects use new i64 slot accessors. Optional calls are parsed as `Call` with
  an `optional` flag and share call type checking; at the time they lowered as
  direct calls (see ADR-0020), and ADR-0027 added indirect calls for function
  values. Shared call-tail parsing lives in `parser/calls.rs`; shared
  instruction result-type logic lives in `ir/results.rs` and is used by both the
  verifier and codegen.
- **Consequences:** Bigint is *not* arbitrary precision; mixing it with `number`
  is diagnosed rather than coerced, and `+ - *` wrap on overflow. Optional calls
  have no null branch because callable values cannot be null (ADR-0027). The unified
  result-type mapping also corrected a pre-existing `And`/`Or` divergence
  between verifier and codegen. Arbitrary precision and nullable callables
  remain out of scope; first-class functions were delivered later in Phase 6 by
  ADR-0027 (optional calls on a function value are still non-nullable).

---

## ADR-0023 — Generics via call-site monomorphization

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 6 adds generics to broaden TypeScript compatibility. NavaScript
  IR has concrete types only — every function has a fixed signature and a named
  LLVM symbol — so there is no representation for an unbound type parameter at
  the backend. A "generic" function lowered with `Opaque` parameters would
  silently miscompile. Generic *type references* also need the checker's
  declaration registry to know a generic's parameter names, which the free
  `resolve_type` helper does not have.
- **Decision:** `Type::TypeParam(String)` represents a parameter inside a
  generic declaration. `Checker::resolve` is the single type resolver: it
  applies declaration substitutions, resolves `Box<T>` references from the
  registered generic interfaces, and emits diagnostics for unknown generics or
  wrong type-argument arity. At a call, declared parameter types are unified with
  actual argument types (or explicit type arguments) to produce a `Subst`, which
  is recorded on the call's span for IR. The IR skips generic templates and emits
  a specialization per distinct type-argument combination on demand, with a
  structural name mangling (`id__number`, `first__number_array`,
  `id__object_1_a_number`) and numeric disambiguation against user-defined
  names. Recursive calls reuse a reserved specialization. Function signatures
  in IR come from the checker's resolved types, not AST annotations. The helpers
  live in private `typechecker/generics.rs`, `ir/generics.rs` and
  `parser/calls.rs` behind the existing seams. Generic type aliases are
  deliberately out of scope.
- **Consequences:** Each instantiation produces a separate native function;
  there is no cross-instantiation sharing or generic code reuse. Constraints,
  higher-kinded types and inference across call boundaries beyond argument
  unification are unsupported. Because monomorphization is driven by recorded
  type arguments, an unbound `TypeParam` is diagnosed instead of reaching
  codegen as `Opaque`.

---

## ADR-0024 — Class layout and static dispatch

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** Phase 6 adds classes and inheritance. NavaScript objects are flat
  arrays of 8-byte slots, and IR functions have concrete signatures. A derived
  instance has more fields than its base, so a base method that indexes `this`
  by position could read a different field than the same index in the derived
  object. Structural class identity also made method dispatch depend on hash-map
  iteration order when two classes shared a field layout.
- **Decision:** Class instances use a nominal `Type::ClassInstance { name,
  fields }`, where `fields` is ordered base-first, inherited fields preceding
  derived ones. `IrType::Object` preserves that order, so every method in a
  hierarchy agrees on slot indices. Method resolution walks the base chain and
  targets the function named `<DeclaringClass>_<method>`; a receiver without
  class identity is diagnosed rather than guessed. Constructors are inherited
  when a derived class declares none, `super(args)` emits a direct call to the
  base constructor on `this`, and methods/constructors lower to named functions
  with `this` as parameter 0. Class checking and shape resolution are split into
  private `typechecker/class_check.rs` and `typechecker/classes.rs`; lowering
  lives in `ir/classes.rs`.
- **Consequences:** Dispatch is static — there are no virtual methods, so a
  base-typed reference calls the base implementation. Field initialisers are
  unsupported; fields are assigned in constructors. Full nominal subtyping and
  virtual dispatch remain out of scope and are recorded in `COMPATIBILITY.md`.

---

## ADR-0025 — Unions, nullable references and narrowing

- **Date:** 2026-11-21
- **Status:** Accepted
- **Context:** ADR-0020 deferred a unions ADR and left optional-chaining results
  typed `T` because no nullable value could be represented. Phase 6 now adds
  `A | B` / `A & B`, but NavaScript values are unboxed and untagged: `number | string`
  has no single native representation, and storing `null` in a numeric slot
  would produce invalid LLVM IR.
- **Decision:** Union and intersection types are normalized and resolved in the
  checker. Assignability for unions is order-independent (every actual member
  must match some target member). The checker rejects unions whose members do
  not share a runtime representation, and rejects nullable primitives; only
  nullable reference types (`T | null`) are lowered. For those, the IR uses the
  reference member's pointer representation and the existing `IsNull`
  instruction, so null tests and narrowing are observable. The checker does not
  yet accept a nullable receiver for `a?.b`, so that branch is emitted but not
  reachable through `?.` (recorded in `COMPATIBILITY.md`). Optional object
  properties resolve to `T | null` and may be omitted; omitted fields are
  initialized to null (references) or zero (scalars). Intersections merge their
  object fields in sorted order so member indices match object literals.
  Narrowing refines symbol types for null tests in `if`, `while`, `&&` and
  `||`; it lives in a private `typechecker/narrowing.rs`.
- **Consequences:** Tagged unions, discriminated narrowing, ternary/post-
  assignment flow analysis and optional parameters remain out of scope and are
  recorded in `COMPATIBILITY.md`. Mixed-representation unions are diagnosed
  rather than silently miscompiled, satisfying the "never silently miscompile"
  working rule.

---

## ADR-0026 — Modules by concatenation

- **Date:** 2026-11-24
- **Status:** Accepted
- **Context:** Phase 6 adds modules. The compiler's seams (`parse`, `check`,
  `lower`, `generate`) all accept one program and one source string; threading
  `FileId` and per-file diagnostics through every phase would be a much larger
  change than this increment warrants.
- **Decision:** The driver resolves the import graph from the entry file before
  compilation. Each file is parsed once to discover its imports, visited paths
  are deduplicated (so cycles and diamonds terminate), extensionless specifiers
  gain `.ns`, and file texts are concatenated in dependency order. The existing
  seams then see one program. `import { ... } from "..."` and `export
  <declaration>` are AST items; exported declarations are collected from
  `export` wrappers and imports are validated against them. `from` stays
  contextual so it remains a valid identifier outside import statements. The
  loader lives in `compiler/driver/src/modules.rs` behind the build pipeline.
- **Consequences:** All files share one namespace — non-exported names are
  visible across files, and two modules declaring the same name collide.
  Diagnostics are rendered against the concatenated source, so line/column
  point into combined text rather than the originating file. Resolution covers
  relative `.ns`/extensionless paths only; `.d.ts`, packages and per-file
  namespaces are out of scope and recorded in `COMPATIBILITY.md`.

---

## ADR-0027 — First-class functions and closures

- **Date:** 2026-11-24
- **Status:** Accepted (supersedes ADR-0019's named-initialiser restriction)
- **Context:** The next Phase 6 priority item after modules is closures and
  higher-order functions (ADR-0015). ADR-0019 lowered arrow initialisers to
  named functions with direct calls, which cannot express a function stored in
  a variable, passed as an argument, returned from a call or placed in a
  collection, nor an arrow that captures a surrounding local. IR had no
  function type and no indirect call.
- **Decision:** A function value is an arena-allocated NavaScript object: slot 0
  holds the code pointer and the remaining slots hold captured bindings in
  sorted name order. `IrType::Function` maps to LLVM `ptr`; `Instruction::ClosureNew`
  allocates the object and `Instruction::CallIndirect` casts slot 0 to the
  statically known function signature and passes the object as a hidden first
  parameter. Captured bindings are shared one-slot cells, so writes from a
  closure and its enclosing frame are mutually visible; nested closures share
  the same cell. `this` is captured by value (it is never reassigned). Named
  functions used as values get zero-capture thunks that adapt the ordinary
  signature to the closure convention. Arrows lower in any expression position;
  the checked function type supplies parameter and return types. All lowering
  stays behind the unchanged `lower` seam, split across `ir/closures.rs`,
  `ir/closure_names.rs` and `ir/bindings.rs` (ADR-0016), and codegen reuses the
  existing object slot accessors with no new runtime ABI.
- **Consequences:** Higher-order functions, returned closures, function values
  in arrays/object fields, nested closures and shared mutable captures run
  natively (`examples/closures.ns`, ADR tests in `tests/`). Calls through a
  non-function value, wrong argument types and unannotated arrow parameters are
  diagnosed. Function values are not nullable, so optional calls still emit no
  null branch; recursion via a captured `const` arrow binding is unsupported
  because the binding is not initialised when the arrow is created — use a
  named function. `console.log` of a function value is diagnosed by the backend
  rather than printing raw bytes. Parenthesised nested function types in return
  annotations need a type alias due to a parser ambiguity.

---

## ADR-0028 — Same-function exceptions

- **Date:** 2026-11-24
- **Status:** Accepted (supersedes ADR-0015's "no exception support" consequence)
- **Context:** The next Phase 6 priority item after closures is exceptions
  (ADR-0015). Full JavaScript exceptions require unwinding across generated
  frames, which the IR and backend do not support; adding `setjmp`/`longjmp` or
  landing pads would be a much larger change than this increment warrants.
- **Decision:** `throw`/`try`/`catch`/`finally` lower as ordinary IR control
  flow inside one function. A `throw` stores its value in the enclosing try's
  typed slot and branches to the catch block; the try body and catch converge
  through `finally` into a join block. A catch whose block has no throwing
  predecessor is omitted so unreachable blocks cannot fail verification. The
  resolved catch type is recorded on the catch binding's span so aliases lower
  to concrete IR types. Exception context is suspended when entering a named
  function or arrow, so a handler from an enclosing function cannot catch a
  throw there. Checking lives in private `typechecker/exceptions.rs` and
  lowering in private `ir/exceptions.rs`, both behind the existing `check` and
  `lower` seams (ADR-0016). No new runtime ABI or LLVM instructions were added.
- **Consequences:** Nested try/catch/finally with same-function throws runs
  natively (`examples/exceptions.ns`). Unsupported cases are diagnosed rather
  than silently miscompiled: throwing without a handler in the same function,
  `throw` in catch/finally (which would bypass `finally`), `return`/`break`/
  `continue` out of protected regions, unannotated catch bindings, and thrown
  values that do not match the catch type. `try` requires a catch clause;
  `finally`-only try and untyped/anonymous `catch (e)` are not supported.
  Runtime bounds/field errors remain fatal (not catchable), and cross-function
  unwinding is deferred until the IR has a real unwind model.

---

## ADR-0029 — Synchronous async/await and promises

- **Date:** 2026-11-24
- **Status:** Accepted (completes the Phase 6 expansion list)
- **Context:** The final Phase 6 item is "Async/await and promises". Full
  JavaScript promises require suspension, an event loop and a microtask queue;
  none exist in the single-threaded native runtime. Adding them would be a
  larger change than this increment, and a misleading partial implementation
  would silently miscompile TypeScript (README working rule).
- **Decision:** `async function` wraps every returned value in a promise object
  with one `__value` slot; `await promise` reads that slot synchronously. The
  layout and wrap/unwrap helpers live in private `ir/promises.rs` behind the
  `lower` seam (ADR-0016). The checker adds a built-in `Promise<T>` generic and
  a `Type::Promise` variant; the resolved type is recorded on function
  signatures, and `Promise<void>` maps to `void` so an async function without a
  return lowers cleanly. `await` is allowed at program scope (module
  concatenation makes it the module body) and in async functions; elsewhere it
  is diagnosed. Only `async function` declarations are parsed.
- **Consequences:** `examples/async_await.ns` runs natively, including promises
  stored in collections. There is **no** suspension, event loop, microtask
  queue, `.then`/`.catch`, `Promise.resolve` or `Promise.all`; `async` arrows
  and methods and out-of-context `await` are diagnosed, not silently accepted.
  These gaps are recorded in `COMPATIBILITY.md` §6. Cross-frame unwinding and a
  real scheduler remain deferred; ADR-0007 keeps this separate from the
  TypeScript-compatibility and native-compilation goals.

---

## Template for new decisions

```
## ADR-00NN — Title
- Date:
- Status: Proposed | Accepted | Superseded by ADR-00NN
- Context:
- Decision:
- Consequences:
```
