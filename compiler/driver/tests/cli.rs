//! Integration tests that exercise the real `nava` binary.
//!
//! The binary is the driver's external interface, so behaviour is asserted
//! there; only pure parse-result assertions stay in the driver's unit tests.

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

fn ok(output: Output) -> String {
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

fn failed(output: Output) -> String {
    assert!(
        !output.status.success(),
        "expected failure, got: {}",
        stdout(&output)
    );
    stderr(&output)
}

#[test]
fn version_prints_semantic_version() {
    let text = ok(nava()
        .arg("--version")
        .output()
        .expect("run nava --version"));
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
    let text = ok(nava().arg("-V").output().expect("run nava -V"));
    assert!(text.starts_with("nava "));
}

#[test]
fn version_rejects_extra_arguments() {
    let text = failed(
        nava()
            .args(["--version", "extra"])
            .output()
            .expect("run nava --version extra"),
    );
    assert!(text.contains("takes no arguments"));
}

#[test]
fn help_prints_usage_and_commands() {
    let text = ok(nava().arg("--help").output().expect("run nava --help"));
    assert!(text.contains("Usage:"), "missing usage: {text}");
    assert!(text.contains("build"), "missing build command: {text}");
    assert!(text.contains("--version"), "missing version flag: {text}");
}

#[test]
fn short_help_flag_works() {
    let text = ok(nava().arg("-h").output().expect("run nava -h"));
    assert!(text.contains("Usage:"));
}

#[test]
fn no_arguments_prints_help() {
    let text = ok(nava().output().expect("run nava with no args"));
    assert!(text.contains("Usage:"));
}

#[test]
fn unknown_option_fails_with_error() {
    let text = failed(nava().arg("--nope").output().expect("run nava --nope"));
    assert!(text.contains("unknown option"));
}

#[test]
fn unknown_command_fails_with_error() {
    let text = failed(
        nava()
            .arg("frobnicate")
            .output()
            .expect("run nava frobnicate"),
    );
    assert!(text.contains("unknown command"));
}

#[test]
fn build_without_input_fails() {
    let text = failed(nava().arg("build").output().expect("run nava build"));
    assert!(text.contains("missing input file"));
}

#[test]
fn build_rejects_multiple_inputs() {
    let text = failed(
        nava()
            .args(["build", "a.ts", "b.ts"])
            .output()
            .expect("run nava build with two inputs"),
    );
    assert!(text.contains("expected exactly one input file"));
}

#[test]
fn build_rejects_option_as_input() {
    let text = failed(
        nava()
            .args(["build", "--oops"])
            .output()
            .expect("run nava build --oops"),
    );
    assert!(text.contains("unknown option"));
}

#[test]
fn build_accepts_a_source_file_without_generating_code() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/fibonacci.ts");
    let text = ok(nava()
        .args(["build", source])
        .output()
        .expect("run nava build examples/fibonacci.ts"));
    assert!(text.contains("not implemented yet"));
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
