//! Command-line parsing and dispatch for the Nava compiler.
//!
//! The parser is deliberately dependency-free: argument handling is simple in
//! Phase 0 and a hand-rolled parser keeps the bootstrap compiler easy to build.

use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
Nava — a TypeScript-compatible language compiled to native machine code

Usage:
  nava [OPTIONS] <COMMAND>

Commands:
  build <FILE.ts>   Compile a TypeScript source file to a native executable
                    (placeholder in Phase 0; code generation arrives in Phase 4)

Options:
  -h, --help        Print this help message
  -V, --version     Print the compiler version

Examples:
  nava --version
  nava --help
  nava build examples/fibonacci.ts
";

/// A successfully parsed invocation, or a description of an invalid one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print the help text.
    Help,
    /// Print the compiler version.
    Version,
    /// Build a source file into a native executable.
    Build { input: PathBuf },
    /// An invalid invocation, with a human-readable reason.
    Error(String),
}

/// Handle a flag that takes no arguments, rejecting any trailing tokens.
fn no_arg_flag<I, S>(mut args: I, flag: &str, command: Command) -> Command
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if args.next().is_some() {
        Command::Error(format!("nava: {flag} takes no arguments"))
    } else {
        command
    }
}

/// Parse command-line arguments (excluding the program name).
///
/// With no arguments, help is shown. Parsing is pure so it can be unit tested
/// without spawning a process.
pub fn parse_args<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut args = args.into_iter();
    let first = match args.next() {
        None => return Command::Help,
        Some(value) => value.as_ref().to_owned(),
    };

    match first.as_str() {
        "-h" | "--help" => no_arg_flag(args, "--help", Command::Help),
        "-V" | "--version" => no_arg_flag(args, "--version", Command::Version),
        "build" => {
            let mut rest = args.map(|value| value.as_ref().to_owned());
            match (rest.next(), rest.next()) {
                (None, _) => Command::Error("nava build: missing input file".to_owned()),
                (Some(_), Some(_)) => {
                    Command::Error("nava build: expected exactly one input file".to_owned())
                }
                (Some(input), None) if input.starts_with('-') => {
                    Command::Error(format!("nava build: unknown option '{input}'"))
                }
                (Some(input), None) => Command::Build {
                    input: PathBuf::from(input),
                },
            }
        }
        other if other.starts_with('-') => {
            Command::Error(format!("nava: unknown option '{other}'"))
        }
        other => Command::Error(format!("nava: unknown command '{other}'")),
    }
}

/// Read the process arguments and return a process exit code.
pub fn run() -> i32 {
    dispatch(parse_args(std::env::args().skip(1)))
}

/// Execute a parsed command. Private: the binary's interface is [`run`].
fn dispatch(command: Command) -> i32 {
    match command {
        Command::Help => {
            print!("{HELP}");
            0
        }
        Command::Version => {
            println!("nava {VERSION}");
            0
        }
        Command::Build { input } => {
            if !input.exists() {
                eprintln!("nava: input file not found: {}", input.display());
                return 1;
            }
            println!(
                "nava: accepted '{}'; native code generation is not implemented yet (Phase 4).",
                input.display()
            );
            0
        }
        Command::Error(message) => {
            eprintln!("{message}");
            eprintln!("Run 'nava --help' for usage.");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            }
        );
    }

    #[test]
    fn help_lists_the_build_command() {
        assert!(HELP.contains("Usage:"));
        assert!(HELP.contains("build"));
        assert!(HELP.contains("--version"));
    }
}
