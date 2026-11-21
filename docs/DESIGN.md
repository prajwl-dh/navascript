# Nava — Module Design (deep modules)

This document records the chosen **interfaces and seams** for the compiler, in
the vocabulary of the codebase-design skill:

- **Module** — anything with an interface and an implementation (function, type, crate, slice).
- **Interface** — everything a caller must know: signature, invariants, ordering, error modes, configuration, performance. Not just the type signature.
- **Seam** — where the interface lives; a place behaviour can change without editing there.
- **Adapter** — a concrete thing satisfying an interface at a seam.
- **Depth** — leverage at the interface: behaviour per unit of interface a caller learns.
- **Leverage / Locality** — capability for callers; concentration of change, bugs and knowledge for maintainers.

Status: **accepted design for Phase 1**, not yet implemented. Revisit when a
second real adapter or consumer appears.

## 1. The deepened module: the front-end seam

**Module:** `nava_parser` (crate `nava-parser`).
**External seam:** `nava_parser::parse(&str) -> ParseOutput` — one call turns
source text into a program plus diagnostics.

```rust
// nava-ast — dependency-free leaf crate (real second consumers: typechecker, IR)
pub struct FileId(pub u32);
pub struct Span { pub file: FileId, pub start: u32, pub end: u32 } // half-open byte offsets
pub struct Program { pub items: Vec<Item> }

// nava-parser — the external seam
pub fn parse(source: &str) -> ParseOutput;

pub struct ParseOutput {
    pub program: Program,          // best-effort / partial AST even on error
    pub diagnostics: Diagnostics,
}

impl Diagnostics {
    pub fn is_empty(&self) -> bool;
    pub fn has_errors(&self) -> bool;
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic>;
    pub fn render(&self, source: &str) -> String; // '<input>:line:col: error: ...' + caret
}

pub struct Diagnostic {
    pub severity: Severity,        // Error | Warning
    pub message: String,
    pub span: Span,
    pub line: u32,                 // 1-based, derived from span
    pub column: u32,               // 1-based, derived from span
}
pub enum Severity { Error, Warning }
```

### Interface contract (part of the interface, not documentation trivia)

- `parse` is **infallible**: no `Result`, no I/O, no configuration; it never
  panics for any `&str` (allocation failure aside).
- It always returns a **total, partially recovered `Program`**. Syntax errors
  become diagnostics; the driver must not proceed to later phases when
  `diagnostics.has_errors()` is true.
- Diagnostics are **deduplicated and sorted by `span.start`**, with stable
  tie-breaking by severity then message. Output is deterministic.
- `line`/`column` are 1-based and derived from byte offsets; `span` is a valid
  half-open range into `source`.
- `ParseOutput` owns all data (no lifetimes in the interface) so it ports
  cleanly to Nava and survives internals refactors.

## 2. Hidden behind the seam (implementation, not interface)

- `nava-lexer` as a **private internal seam** of `nava_parser`: byte-offset
  tokenization, trivia handling, keyword table, token spans.
- Recursive-descent expression parsing with precedence, statement/declaration
  parsing, panic-mode recovery with sync sets.
- Span attachment, span → line/column mapping via a line-start table.
- Diagnostic accumulation, deduplication, ordering and caret rendering.

`nava-lexer` is not driver-visible; `nava-ast` stays a **public dependency-free
leaf** because the type checker and IR lowering are real second consumers of
`Program`, `Span` and `FileId`.

## 3. Dependency strategy and adapters

Lexing, parsing and AST construction are **in-process pure computation**
(Dependency category 1): deepen and merge directly, test through the interface.
No port is needed for file reading — source is `&str`; an FS adapter would be a
single, hypothetical adapter. There is **no `DiagnosticSink` trait yet**:
unit tests read the `ParseOutput.diagnostics` field, so there is only one
adapter today.
Introduce the trait when a second real adapter exists (e.g. an LSP publishing
diagnostics or a type checker streaming into the same sink).

## 4. What was considered and rejected

| Option | Why not |
| --- | --- |
| Facade in `nava_driver::front_end` | Couples the front end to the CLI crate; a future language server would depend on the driver. |
| `parse(&[Source], &mut dyn DiagnosticSink)` now | Larger interface before a second adapter or a module system exists. |
| `FileId`-threading through every phase now | Phase 6/7 module support is a real need but not a current one; `Span` already carries `FileId(0)` so the change is contained when it arrives. |
| Flags/options for recovery policy or incremental ranges | Speculative generality; use a separate function when a real second caller appears. |

## 5. Consequences for other modules

- **Driver (`cli.rs`)** reads the file, calls `nava_parser::parse(&source)`,
  renders `diagnostics`, checks `has_errors()`, then passes `program` to the
  next phase. It never imports lexer or AST internals beyond the leaf types.
- **Type checker (Phase 2)** consumes `&Program` and reports into the same
  `Diagnostics` shape; it does not change `parse`'s signature.
- **IR lowering (Phase 3)** consumes the typed program; no LLVM types cross the
  front-end seam.
- **Tests** use the same `parse` seam as the driver; any AST-shape assertions
  live behind an internal seam private to `nava-parser`'s own tests.
- **Self-hosting (Phase 9)** ports plain owned types and one function — no
  traits, generics or borrowed AST in the interface.

## 6. Design rules to keep

1. Keep the interface small; hide complexity in the implementation.
2. No seam without two real adapters (one adapter = hypothetical seam).
3. The interface is the test surface: tests assert observable results through
   `parse`, not internal state.
4. Replace shallow-module tests when a deepened interface absorbs them; do not
   layer tests on both.
5. Add extension points only when a second caller actually exists.
