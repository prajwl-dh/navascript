//! End-to-end optimizer tests: optimized binaries must match the baseline.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn build_binary(name: &str, flags: &[&str]) -> Option<PathBuf> {
    if !navascript_driver::build::native_tools_available() {
        eprintln!("skipping native optimizer test: LLVM toolchain not available");
        return None;
    }
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name);
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let output = std::env::temp_dir().join(format!(
        "navascript-opt-{}-{name}",
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let status = Command::new(env!("CARGO_BIN_EXE_navascript"))
        .arg("build")
        .args(flags)
        .arg(&source)
        .arg("-o")
        .arg(&output)
        .status()
        .expect("run navascript build");
    assert!(status.success(), "build failed for {name} with {flags:?}");
    Some(output)
}

fn run(binary: &Path) -> String {
    let output = Command::new(binary).output().expect("run program");
    assert!(output.status.success(), "program failed");
    String::from_utf8(output.stdout).expect("utf-8 output")
}

#[test]
fn optimized_executables_match_the_unoptimized_baseline() {
    for name in [
        "numbers.ns",
        "fibonacci.ns",
        "classes.ns",
        "closures.ns",
        "exceptions.ns",
        "async_await.ns",
    ] {
        let Some(baseline) = build_binary(name, &["--no-opt"]) else {
            return;
        };
        let Some(optimized) = build_binary(name, &["--release"]) else {
            return;
        };
        assert_eq!(
            run(&baseline),
            run(&optimized),
            "different output for {name}"
        );
        let _ = std::fs::remove_file(baseline);
        let _ = std::fs::remove_file(optimized);
    }
}
