//! Unit-level tests for the CLI's pure argument parser.

use std::path::PathBuf;

use nava_driver::cli::{Command, HELP, parse_args};

#[test]
fn no_arguments_shows_help() {
    assert_eq!(parse_args(std::iter::empty::<&str>()), Command::Help);
}

#[test]
fn parses_build_with_one_input() {
    assert_eq!(
        parse_args(["build", "examples/fibonacci.ts"]),
        Command::Build {
            input: PathBuf::from("examples/fibonacci.ts"),
            output: None,
            emit_ir: false,
            release: false,
        }
    );
}

#[test]
fn help_lists_the_build_command() {
    assert!(HELP.contains("Usage:"));
    assert!(HELP.contains("build"));
    assert!(HELP.contains("--version"));
}
