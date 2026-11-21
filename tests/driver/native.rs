//! End-to-end tests: compile a `.ts` file to a native executable and run it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn output_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("nava-native-{}-{name}", std::process::id()))
}

fn compile(source: &str, name: &str, extra: &[&str]) -> Option<PathBuf> {
    if !nava_driver::build::native_tools_available() {
        eprintln!("skipping native test: LLVM toolchain not available");
        return None;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(source);
    let output = output_path(name);
    let status = Command::new(env!("CARGO_BIN_EXE_nava"))
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .args(extra)
        .status()
        .expect("run nava build");
    assert!(status.success(), "nava build failed for {name}");
    Some(output)
}

#[test]
fn fibonacci_prints_55_natively() {
    let Some(binary) = compile("fibonacci.ts", "fib", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run fibonacci");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "55",
        "native fibonacci output"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn multiple_functions_with_distinct_value_id_ranges_compile() {
    let Some(binary) = compile("multifunction.ts", "multi", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run multifunction program");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn booleans_print_as_true_or_false() {
    let Some(binary) = compile("booleans.ts", "booleans", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run booleans program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "false\ntrue\nfalse\ntrue\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn numbers_print_without_precision_loss() {
    let Some(binary) = compile("numbers.ts", "numbers", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run numbers program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1234567.89\n0.00001234\n1.5\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn strings_run_natively() {
    let Some(binary) = compile("strings.ts", "strings", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run strings program");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "hello world\n11\nh\ntrue\n"
    );
    let _ = std::fs::remove_file(binary);
}

#[test]
fn arrays_run_natively() {
    let Some(binary) = compile("arrays.ts", "arrays", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run arrays program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n42\n3\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn objects_run_natively() {
    let Some(binary) = compile("objects.ts", "objects", &[]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run objects program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\n9\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn dynamic_indices_and_string_coercion_run_natively() {
    let Some(binary) = compile("dynamic_access.ts", "dynamic", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run dynamic_access program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "20\nb\na1\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn object_fields_keep_their_declared_order() {
    let Some(binary) = compile("object_fields.ts", "object-fields", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run object_fields program");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "nava\n2\ntrue\n");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn out_of_bounds_index_is_a_runtime_error() {
    let Some(binary) = compile("runtime_error.ts", "runtime-error", &[]) else {
        return;
    };
    let output = Command::new(&binary)
        .output()
        .expect("run runtime_error program");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("array index out of bounds"), "{stderr}");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn release_mode_prints_the_same_result() {
    let Some(binary) = compile("fibonacci.ts", "fib-release", &["--release"]) else {
        return;
    };
    let output = Command::new(&binary).output().expect("run fibonacci");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "55");
    let _ = std::fs::remove_file(binary);
}

#[test]
fn emit_ir_prints_llvm_ir_without_linking() {
    if !nava_driver::build::native_tools_available() {
        return;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/fibonacci.ts");
    let output = Command::new(env!("CARGO_BIN_EXE_nava"))
        .args(["build", "--emit-ir"])
        .arg(source)
        .output()
        .expect("run nava build --emit-ir");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("define double @nava_fibonacci"), "{text}");
    assert!(text.contains("target triple"), "{text}");
}
