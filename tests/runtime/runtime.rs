//! Compile and run the C runtime's unit tests.

use std::path::Path;
use std::process::Command;

#[test]
fn runtime_unit_tests() {
    if !navascript_driver::build::native_tools_available() {
        eprintln!("skipping: native toolchain not available");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("tests/runtime/runtime_test.c");
    let workdir = root.join("runtime");
    let binary =
        std::env::temp_dir().join(format!("navascript-runtime-test-{}", std::process::id()));
    let mut command = Command::new("clang");
    command
        .args(["-std=c11", "-Wall", "-Wextra", "-O0"])
        .arg(&source);
    // Compile every runtime source except the entry-point wrapper, which
    // would conflict with this test's own main.
    for (_, path, _) in navascript_codegen::RUNTIME_FILES {
        if path.ends_with(".c") && *path != "navascript_runtime.c" {
            command.arg(workdir.join(path));
        }
    }
    let status = command
        .arg("-I")
        .arg(&workdir)
        .arg("-o")
        .arg(&binary)
        .status()
        .expect("compile C runtime tests");
    assert!(status.success(), "C runtime test compilation failed");

    let output = Command::new(&binary).output().expect("run C runtime tests");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("runtime unit tests passed"), "{stdout}");
    let _ = std::fs::remove_file(binary);
}
