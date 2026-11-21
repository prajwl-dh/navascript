//! Command-line parsing and dispatch for the Nava compiler.
//!
//! The parser is deliberately dependency-free: argument handling is simple in
//! Phase 0 and a hand-rolled parser keeps the bootstrap compiler easy to build.

use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Help text shown for `--help` and for an empty invocation.
pub const HELP: &str = "\
Nava — a TypeScript-compatible language compiled to native machine code

Usage:
  nava [OPTIONS] <COMMAND>

Commands:
  build <FILE.ts>   Parse, type-check and lower a source file to Nava IR
                    (native code generation arrives in Phase 4)

Options:
  -h, --help        Print this help message
  -V, --version     Print the compiler version
      --emit-ir     With build, print the generated Nava IR

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
    /// Lower a source file to Nava IR; optionally print it.
    Build { input: PathBuf, emit_ir: bool },
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
            let rest: Vec<String> = args.map(|value| value.as_ref().to_owned()).collect();
            let emit_ir = rest.iter().any(|arg| arg == "--emit-ir");
            let inputs: Vec<&String> = rest
                .iter()
                .filter(|arg| arg.as_str() != "--emit-ir")
                .collect();
            match inputs.as_slice() {
                [] => Command::Error("nava build: missing input file".to_owned()),
                [_] if inputs[0].starts_with('-') => {
                    Command::Error(format!("nava build: unknown option '{}'", inputs[0]))
                }
                [input] => Command::Build {
                    input: PathBuf::from(input),
                    emit_ir,
                },
                _ => Command::Error("nava build: expected exactly one input file".to_owned()),
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
        Command::Build { input, emit_ir } => {
            let source = match std::fs::read_to_string(&input) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("nava: could not read '{}': {error}", input.display());
                    return 1;
                }
            };
            let parsed = nava_parser::parse(&source);
            if parsed.diagnostics.has_errors() {
                eprint!("{}", parsed.diagnostics.render(&source));
                return 1;
            }
            let checked = nava_typechecker::check(&parsed.program, &source);
            if checked.diagnostics.has_errors() {
                eprint!("{}", checked.diagnostics.render(&source));
                return 1;
            }
            let lowered = nava_ir::lower(&parsed.program, &checked.typed, &source);
            if lowered.diagnostics.has_errors() {
                eprint!("{}", lowered.diagnostics.render(&source));
                return 1;
            }
            if emit_ir {
                print!("{}", nava_ir::print(&lowered.module));
            } else {
                println!(
                    "nava: lowered '{}' to {} IR function(s); native code generation arrives in Phase 4.",
                    input.display(),
                    lowered.module.functions.len()
                );
            }
            0
        }
        Command::Error(message) => {
            eprintln!("{message}");
            eprintln!("Run 'nava --help' for usage.");
            2
        }
    }
}
