# Nava Runtime ABI

The native backend emits calls into `runtime/nava_runtime.c`. This is the ABI
contract between `nava-codegen` and the C runtime; keep both in sync.

## Memory (ADR-0005)

- One static, 8-byte-aligned arena of 16 MiB. `nava_alloc(bytes)` bumps a
  program-lifetime cursor and zero-fills the allocation.
- Nothing is reclaimed before process exit. Exhaustion prints
  `nava: runtime arena exhausted` and exits with status 1.
- All logical slots are 8 bytes. Boolean slots use one byte inside an 8-byte
  slot; the others store `double` or pointer values.

## Entry point

- `extern void nava_main(void)` is called by the C `main`. The driver emits a
  stub `nava_main` when the program has no top-level statements.
- Define `NAVA_NO_MAIN` when including the runtime in unit tests.

## Functions

| Signature | Purpose |
| --- | --- |
| `void *nava_alloc(size_t)` | arena allocation |
| `void nava_runtime_error(const char *)` | fatal diagnostic, exit 1 |
| `void nava_format_number(double, char *, size_t)` | JS-like number formatting |
| `char *nava_number_to_string(double)` | numeric coercion for `+` |
| `char *nava_string_concat(const char *, const char *)` | string concatenation |
| `i32 nava_string_equals(const char *, const char *)` | string content equality |
| `i64 nava_string_length(const char *)` | byte length |
| `char *nava_string_char_at(const char *, i64)` | single-character string |
| `ptr nava_array_new(i64)` | allocate an array of 8-byte slots |
| `nava_array_set_{f64,bool,ptr}(ptr, i64, value)` | bounds-checked slot write |
| `nava_array_get_{f64,bool,ptr}(ptr, i64)` | bounds-checked slot read |
| `i64 nava_array_length(ptr)` | array length |
| `ptr nava_object_new(i64, ptr)` | object with NUL-separated field-name table |
| `nava_object_set_{f64,bool,ptr}(ptr, i64, value)` | bounds-checked field write |
| `nava_object_get_{f64,bool,ptr}(ptr, i64)` | bounds-checked field read |
| `nava_console_log_{f64,bool,str}(value)` | stdout printing |
| `void nava_format_number(...)` | shared formatting helper |

Field names are stored for future reflection/debugging; field access by the
compiler uses the index in the inferred type's sorted field list. Bounds are
checked on every access.

## Tests

`tests/runtime/runtime_test.c` is compiled by `clang` and run by
`tests/runtime/runtime.rs`; it covers alignment, strings, number coercion,
array/object slot access and cross-slot isolation.
