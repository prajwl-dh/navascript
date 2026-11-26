//! Runtime source bundle and LLVM declarations.

use std::fmt::Write;

use navascript_ir::IrType;

/// (flattened output name, repository-relative path, source text).
pub const RUNTIME_FILES: &[(&str, &str, &str)] = &[
    (
        "navascript_runtime.h",
        "navascript_runtime.h",
        include_str!("../../../runtime/navascript_runtime.h"),
    ),
    (
        "memory.c",
        "memory/memory.c",
        include_str!("../../../runtime/memory/memory.c"),
    ),
    (
        "strings.c",
        "strings/strings.c",
        include_str!("../../../runtime/strings/strings.c"),
    ),
    (
        "arrays.c",
        "arrays/arrays.c",
        include_str!("../../../runtime/arrays/arrays.c"),
    ),
    (
        "objects.c",
        "objects/objects.c",
        include_str!("../../../runtime/objects/objects.c"),
    ),
    (
        "bigint.c",
        "bigint/bigint.c",
        include_str!("../../../runtime/bigint/bigint.c"),
    ),
    ("io.c", "io/io.c", include_str!("../../../runtime/io/io.c")),
    (
        "console.c",
        "console/console.c",
        include_str!("../../../runtime/console/console.c"),
    ),
    (
        "navascript_runtime.c",
        "navascript_runtime.c",
        include_str!("../../../runtime/navascript_runtime.c"),
    ),
];

pub(super) const RUNTIME_DECLS: &[&str] = &[
    "declare double @llvm.pow.f64(double, double) nounwind readnone",
    "declare void @navascript_console_log_f64(double) nounwind",
    "declare void @navascript_console_log_bool(i32) nounwind",
    "declare void @navascript_console_log_str(ptr) nounwind",
    "declare void @navascript_console_log_i64(i64) nounwind",
    "declare i64 @navascript_bigint_div(i64, i64) nounwind",
    "declare i64 @navascript_bigint_mod(i64, i64) nounwind",
    "declare i64 @navascript_bigint_pow(i64, i64) nounwind",
    "declare void @navascript_runtime_error(ptr) nounwind",
    "declare ptr @navascript_string_concat(ptr, ptr) nounwind",
    "declare ptr @navascript_number_to_string(double) nounwind",
    "declare i32 @navascript_string_equals(ptr, ptr) nounwind",
    "declare i64 @navascript_string_length(ptr) nounwind",
    "declare ptr @navascript_string_char_at(ptr, i64) nounwind",
    "declare ptr @navascript_array_new(i64) nounwind",
    "declare void @navascript_array_set_f64(ptr, i64, double) nounwind",
    "declare double @navascript_array_get_f64(ptr, i64) nounwind",
    "declare void @navascript_array_set_i64(ptr, i64, i64) nounwind",
    "declare i64 @navascript_array_get_i64(ptr, i64) nounwind",
    "declare void @navascript_array_set_bool(ptr, i64, i32) nounwind",
    "declare i32 @navascript_array_get_bool(ptr, i64) nounwind",
    "declare void @navascript_array_set_ptr(ptr, i64, ptr) nounwind",
    "declare ptr @navascript_array_get_ptr(ptr, i64) nounwind",
    "declare i64 @navascript_array_length(ptr) nounwind",
    "declare ptr @navascript_object_new(i64, ptr) nounwind",
    "declare void @navascript_object_set_f64(ptr, i64, double) nounwind",
    "declare double @navascript_object_get_f64(ptr, i64) nounwind",
    "declare void @navascript_object_set_i64(ptr, i64, i64) nounwind",
    "declare i64 @navascript_object_get_i64(ptr, i64) nounwind",
    "declare void @navascript_object_set_bool(ptr, i64, i32) nounwind",
    "declare i32 @navascript_object_get_bool(ptr, i64) nounwind",
    "declare void @navascript_object_set_ptr(ptr, i64, ptr) nounwind",
    "declare ptr @navascript_object_get_ptr(ptr, i64) nounwind",
    // Internal helpers declared for ABI completeness (ADR-0016 audit).
    "declare ptr @navascript_alloc(i64) nounwind",
    "declare void @navascript_format_number(double, ptr, i64) nounwind",
    // Phase 9 stdlib: file I/O, process arguments and collection helpers.
    "declare ptr @navascript_read_file(ptr) nounwind",
    "declare void @navascript_write_file(ptr, ptr) nounwind",
    "declare ptr @navascript_get_argv() nounwind",
    "declare ptr @navascript_str_slice(ptr, double, double) nounwind",
    "declare double @navascript_str_index_of(ptr, ptr) nounwind",
    "declare double @navascript_str_compare(ptr, ptr) nounwind",
    "declare double @navascript_str_char_code_at(ptr, double) nounwind",
    "declare i64 @navascript_number_index(double)",
    "declare ptr @navascript_array_push_number(ptr, double) nounwind",
    "declare ptr @navascript_array_push_string(ptr, ptr) nounwind",
];

pub(super) fn escape_llvm_string(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'\\' => out.push_str("\\5C"),
            b'"' => out.push_str("\\22"),
            b'\n' => out.push_str("\\0A"),
            b'\t' => out.push_str("\\09"),
            b'\r' => out.push_str("\\0D"),
            0x20..=0x7E => out.push(byte as char),
            other => {
                let _ = write!(out, "\\{other:02X}");
            }
        }
    }
    out
}

/// Generate textual LLVM IR for a verified NavaScript module.
pub(super) fn llvm_string_bytes(encoded: &str) -> usize {
    let chars: Vec<char> = encoded.chars().collect();
    let mut index = 0;
    let mut bytes = 0;
    while index < chars.len() {
        if chars[index] == '\\' && index + 2 < chars.len() {
            index += 3;
        } else {
            index += 1;
        }
        bytes += 1;
    }
    bytes
}

pub(super) fn encode_field_names(fields: &[(String, IrType)]) -> String {
    let mut encoded = String::new();
    for (name, _) in fields {
        encoded.push_str(&escape_llvm_string(name));
        encoded.push_str("\\00");
    }
    encoded
}
