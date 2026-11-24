# NavaScript Runtime ABI

The native backend emits calls into the C runtime sources under `runtime/`:
`navascript_runtime.h` declares the ABI, `navascript_runtime.c` is only the process entry
point, and `memory/`, `bigint/`, `strings/`, `arrays/`, `objects/` and
`console/` each implement one concern. This is the contract between `navascript-codegen` and the
runtime; keep both in sync. `navascript-codegen::RUNTIME_FILES` embeds these sources
for the driver's build pipeline.

## Memory (ADR-0005)

- One static, 8-byte-aligned arena of 16 MiB. `navascript_alloc(bytes)` bumps a
  program-lifetime cursor and zero-fills the allocation.
- Nothing is reclaimed before process exit. Exhaustion prints
  `navascript: runtime arena exhausted` and exits with status 1.
- All logical slots are 8 bytes. Boolean slots use one byte inside an 8-byte
  slot; the others store `double`, `int64_t` (bigint) or pointer values.

## Entry point

- `extern void navascript_main(void)` is called by the C `main` in
  `runtime/navascript_runtime.c`. The driver emits a stub `navascript_main` when the program
  has no top-level statements.
- Unit tests compile the individual sources except `navascript_runtime.c` and define
  their own `main`, so no preprocessor guard is needed.

## Functions

| Signature | Purpose |
| --- | --- |
| `void *navascript_alloc(size_t)` | arena allocation |
| `void navascript_runtime_error(const char *)` | fatal diagnostic, exit 1 |
| `void navascript_format_number(double, char *, size_t)` | JS-like number formatting |
| `char *navascript_number_to_string(double)` | numeric coercion for `+` |
| `i64 navascript_bigint_div(i64, i64)` | checked bigint division (zero/overflow diagnosed) |
| `i64 navascript_bigint_mod(i64, i64)` | checked bigint modulo (zero divisor diagnosed) |
| `i64 navascript_bigint_pow(i64, i64)` | checked bigint power (negative exponent/overflow diagnosed) |
| `char *navascript_string_concat(const char *, const char *)` | string concatenation |
| `i32 navascript_string_equals(const char *, const char *)` | string content equality |
| `i64 navascript_string_length(const char *)` | byte length |
| `char *navascript_string_char_at(const char *, i64)` | single-character string |
| `ptr navascript_array_new(i64)` | allocate an array of 8-byte slots |
| `navascript_array_set_{f64,i64,bool,ptr}(ptr, i64, value)` | bounds-checked slot write |
| `navascript_array_get_{f64,i64,bool,ptr}(ptr, i64)` | bounds-checked slot read |
| `i64 navascript_array_length(ptr)` | array length |
| `ptr navascript_object_new(i64, ptr)` | object with NUL-separated field-name table |
| `navascript_object_set_{f64,i64,bool,ptr}(ptr, i64, value)` | bounds-checked field write |
| `navascript_object_get_{f64,i64,bool,ptr}(ptr, i64)` | bounds-checked field read |
| `navascript_console_log_{f64,i64,bool,str}(value)` | stdout printing |

Field names are stored for future reflection/debugging; field access by the
compiler uses the index in the inferred type's sorted field list. Bounds are
checked on every access.

## Tests

`tests/runtime/runtime_test.c` is compiled by `clang` and run by
`tests/runtime/runtime.rs`; it covers alignment, strings, number coercion,
array/object slot access (including bigint slots), bigint div/mod/pow and
cross-slot isolation.
