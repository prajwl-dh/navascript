//! Integration tests that exercise the real `nava` binary.

use std::process::{Command, Output};

fn nava() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nava"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn version_prints_semantic_version() {
    let output = nava()
        .arg("--version")
        .output()
        .expect("run nava --version");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.starts_with("nava "), "unexpected output: {text}");
    let version = text.split_whitespace().nth(1).expect("version");
    assert_eq!(
        version.split('.').count(),
        3,
        "unexpected version: {version}"
    );
}

#[test]
fn short_version_flag_works() {
    let output = nava().arg("-V").output().expect("run nava -V");
    assert!(output.status.success());
    assert!(stdout(&output).starts_with("nava "));
}

#[test]
fn help_prints_usage_and_commands() {
    let output = nava().arg("--help").output().expect("run nava --help");
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("Usage:"), "missing usage: {text}");
    assert!(text.contains("build"), "missing build command: {text}");
    assert!(text.contains("--version"), "missing version flag: {text}");
}

#[test]
fn no_arguments_prints_help() {
    let output = nava().output().expect("run nava with no args");
    assert!(output.status.success());
    assert!(stdout(&output).contains("Usage:"));
}

#[test]
fn unknown_option_fails_with_error() {
    let output = nava().arg("--nope").output().expect("run nava --nope");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown option"));
}

#[test]
fn build_without_input_fails() {
    let output = nava().arg("build").output().expect("run nava build");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing input file"));
}

#[test]
fn build_accepts_a_source_file_without_generating_code() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/fibonacci.ts");
    let output = nava()
        .args(["build", source])
        .output()
        .expect("run nava build examples/fibonacci.ts");
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("not implemented yet"));
}

#[test]
fn build_rejects_a_missing_file() {
    let output = nava()
        .args(["build", "does-not-exist.ts"])
        .output()
        .expect("run nava build on missing file");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("input file not found"));
}
