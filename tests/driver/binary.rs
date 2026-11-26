//! Integration tests that exercise the real `navascript` binary.
//!
//! The binary is the driver's external interface, so behaviour is asserted
//! here; only pure parse-result assertions stay in `tests/driver/cli.rs`.

use std::process::{Command, Output};

fn navascript() -> Command {
    Command::new(env!("CARGO_BIN_EXE_navascript"))
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
    let text = ok(navascript()
        .arg("--version")
        .output()
        .expect("run navascript --version"));
    assert!(text.starts_with("navascript "), "unexpected output: {text}");
    let version = text.split_whitespace().nth(1).expect("version");
    assert_eq!(
        version.split('.').count(),
        3,
        "unexpected version: {version}"
    );
}

#[test]
fn short_version_flag_works() {
    let text = ok(navascript().arg("-V").output().expect("run navascript -V"));
    assert!(text.starts_with("navascript "));
}

#[test]
fn version_rejects_extra_arguments() {
    let text = failed(
        navascript()
            .args(["--version", "extra"])
            .output()
            .expect("run navascript --version extra"),
    );
    assert!(text.contains("takes no arguments"));
}

#[test]
fn help_prints_usage_and_commands() {
    let text = ok(navascript()
        .arg("--help")
        .output()
        .expect("run navascript --help"));
    assert!(text.contains("Usage:"), "missing usage: {text}");
    assert!(text.contains("build"), "missing build command: {text}");
    assert!(text.contains("--version"), "missing version flag: {text}");
}

#[test]
fn short_help_flag_works() {
    let text = ok(navascript().arg("-h").output().expect("run navascript -h"));
    assert!(text.contains("Usage:"));
}

#[test]
fn no_arguments_prints_help() {
    let text = ok(navascript().output().expect("run navascript with no args"));
    assert!(text.contains("Usage:"));
}

#[test]
fn unknown_option_fails_with_error() {
    let text = failed(
        navascript()
            .arg("--nope")
            .output()
            .expect("run navascript --nope"),
    );
    assert!(text.contains("unknown option"));
}

#[test]
fn unknown_command_fails_with_error() {
    let text = failed(
        navascript()
            .arg("frobnicate")
            .output()
            .expect("run navascript frobnicate"),
    );
    assert!(text.contains("unknown command"));
}

#[test]
fn build_without_input_fails() {
    let text = failed(
        navascript()
            .arg("build")
            .output()
            .expect("run navascript build"),
    );
    assert!(text.contains("missing input file"));
}

#[test]
fn build_rejects_multiple_inputs() {
    let text = failed(
        navascript()
            .args(["build", "a.ns", "b.ns"])
            .output()
            .expect("run navascript build with two inputs"),
    );
    assert!(text.contains("expected exactly one input file"));
}

#[test]
fn build_rejects_option_as_input() {
    let text = failed(
        navascript()
            .args(["build", "--oops"])
            .output()
            .expect("run navascript build --oops"),
    );
    assert!(text.contains("unknown option"));
}

#[test]
fn build_emits_ir_on_request() {
    if !navascript_driver::build::native_tools_available() {
        eprintln!("skipping: native toolchain not available");
        return;
    }
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/fibonacci.ns");
    let text = ok(navascript()
        .args(["build", source, "--emit-ir"])
        .output()
        .expect("run navascript build --emit-ir"));
    assert!(
        text.contains("define double @navascript_fibonacci"),
        "{text}"
    );
    assert!(text.contains("define void @navascript_main"), "{text}");
}

#[test]
fn build_produces_a_native_executable() {
    if !navascript_driver::build::native_tools_available() {
        eprintln!("skipping: native toolchain not available");
        return;
    }
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/fibonacci.ns");
    let output = std::env::temp_dir().join(format!("navascript-cli-{}", std::process::id()));
    let text = ok(navascript()
        .args(["build", source, "-o"])
        .arg(&output)
        .output()
        .expect("run navascript build examples/fibonacci.ns"));
    assert!(text.contains("built"), "{text}");
    assert!(output.is_file(), "expected a native executable");
    let _ = std::fs::remove_file(output);
}

#[test]
fn build_reports_diagnostics_for_a_broken_file() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/bad_syntax.ns");
    let text = failed(
        navascript()
            .args(["build", source])
            .output()
            .expect("run navascript build examples/bad_syntax.ns"),
    );
    assert!(text.contains("error:"), "{text}");
    assert!(text.contains("1:14"), "{text}");
    assert!(text.contains("^"), "{text}");
}

#[test]
fn build_reports_type_errors() {
    let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples/type_error.ns");
    let text = failed(
        navascript()
            .args(["build", source])
            .output()
            .expect("run navascript build examples/type_error.ns"),
    );
    assert!(text.contains("cannot assign"), "{text}");
    assert!(text.contains("number"), "{text}");
}

#[test]
fn build_rejects_a_missing_file() {
    let output = navascript()
        .args(["build", "does-not-exist.ns"])
        .output()
        .expect("run navascript build on missing file");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("could not read"));
}

#[test]
fn user_main_does_not_collide_with_the_synthetic_entry() {
    if !navascript_driver::build::native_tools_available() {
        return;
    }
    let directory = std::env::temp_dir().join(format!("nava-main-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&directory);
    let source = directory.join("program.ns");
    std::fs::write(
        &source,
        "function main(): void { console.log(1); }\nconsole.log(2);\nmain();\n",
    )
    .expect("write source");
    let binary = directory.join("program-bin");
    let output = navascript()
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("build program with a user main");
    assert!(output.status.success(), "{}", stderr(&output));
    let run = Command::new(&binary).output().expect("run program");
    assert_eq!(stdout(&run), "2\n1\n");
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn default_output_refuses_to_overwrite_the_source_file() {
    let directory = std::env::temp_dir().join(format!("nava-overwrite-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&directory);
    let source = directory.join("prog");
    let original = "console.log(1);\n";
    std::fs::write(&source, original).expect("write source");
    let output = navascript()
        .arg("build")
        .arg("prog")
        .current_dir(&directory)
        .output()
        .expect("build with default output name");
    assert!(
        !output.status.success(),
        "build must refuse to overwrite source"
    );
    assert!(stderr(&output).contains("overwrite"), "{}", stderr(&output));
    assert_eq!(
        std::fs::read_to_string(&source).expect("read source"),
        original
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn user_names_cannot_collide_with_generated_closures() {
    if !navascript_driver::build::native_tools_available() {
        return;
    }
    let directory = std::env::temp_dir().join(format!("nava-closure-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&directory);
    let source = directory.join("program.ns");
    std::fs::write(
        &source,
        "function arrow_0(): number { return 7; }\nlet f = (x: number): number => x + 1;\nconsole.log(arrow_0());\nconsole.log(f(2));\n",
    )
    .expect("write source");
    let binary = directory.join("program-bin");
    let output = navascript()
        .arg("build")
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("build program with a generated-name-like function");
    assert!(output.status.success(), "{}", stderr(&output));
    let run = Command::new(&binary).output().expect("run program");
    assert_eq!(stdout(&run), "7\n3\n");
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn diagnostics_name_the_originating_module() {
    let directory = std::env::temp_dir().join(format!("nava-files-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&directory);
    let main = directory.join("main.ns");
    let math = directory.join("math.ns");
    std::fs::write(
        &main,
        "import { square } from \"./math\";\nconsole.log(square(2));\n",
    )
    .expect("write main");
    std::fs::write(
        &math,
        "export function square(x: number): number {\n  let bad: number = \"oops\";\n  return x;\n}\n",
    )
    .expect("write math");
    let output = navascript()
        .arg("build")
        .arg(&main)
        .arg("-o")
        .arg(directory.join("program-bin"))
        .output()
        .expect("build multi-file program");
    assert!(!output.status.success());
    let text = stderr(&output);
    assert!(
        text.contains("math.ns:2:"),
        "unexpected diagnostics: {text}"
    );
    assert!(!text.contains("<input>"), "unexpected diagnostics: {text}");
    let _ = std::fs::remove_dir_all(&directory);
}
