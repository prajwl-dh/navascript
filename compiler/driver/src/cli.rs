//! Command-line parsing and dispatch for the NavaScript compiler.
//!
//! The parser is deliberately dependency-free: argument handling is simple in
//! Phase 0 and a hand-rolled parser keeps the bootstrap compiler easy to build.

use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Help text shown for `--help` and for an empty invocation.
pub const HELP: &str = "\
NavaScript — a TypeScript-compatible language compiled to native machine code

Usage:
  navascript [OPTIONS] <COMMAND>

Commands:
  build <FILE.ns>   Compile a source file to a native macOS executable

Options:
  -h, --help        Print this help message
  -V, --version     Print the compiler version
  -o, --output PATH Output executable path (default: source file stem)
      --release     Enable aggressive IR + LLVM optimizations (default: debug)
      --no-opt      Build the unoptimized baseline (for benchmarks)
      --emit-ir     With build, print LLVM IR instead of compiling

Examples:
  navascript --version
  navascript --help
  navascript build examples/fibonacci.ns -o fibonacci
  navascript build --emit-ir examples/fibonacci.ns
";

/// A successfully parsed invocation, or a description of an invalid one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print the help text.
    Help,
    /// Print the compiler version.
    Version,
    /// Compile a source file to a native executable (or print LLVM IR).
    Build {
        input: PathBuf,
        output: Option<PathBuf>,
        emit_ir: bool,
        release: bool,
        no_opt: bool,
    },
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
        Command::Error(format!("navascript: {flag} takes no arguments"))
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
            let mut emit_ir = false;
            let mut release = false;
            let mut no_opt = false;
            let mut output = None;
            let mut inputs = Vec::new();
            let mut iter = rest.iter();
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--emit-ir" => emit_ir = true,
                    "--release" => release = true,
                    "--no-opt" => no_opt = true,
                    "-o" | "--output" => match iter.next() {
                        Some(path) => output = Some(PathBuf::from(path)),
                        None => {
                            return Command::Error(format!("navascript build: {arg} needs a path"));
                        }
                    },
                    other if other.starts_with('-') => {
                        return Command::Error(format!(
                            "navascript build: unknown option '{other}'"
                        ));
                    }
                    other => inputs.push(other.to_owned()),
                }
            }
            match inputs.as_slice() {
                [] => Command::Error("navascript build: missing input file".to_owned()),
                [input] => Command::Build {
                    input: PathBuf::from(input),
                    output,
                    emit_ir,
                    release,
                    no_opt,
                },
                _ => Command::Error("navascript build: expected exactly one input file".to_owned()),
            }
        }
        other if other.starts_with('-') => {
            Command::Error(format!("navascript: unknown option '{other}'"))
        }
        other => Command::Error(format!("navascript: unknown command '{other}'")),
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
            println!("navascript {VERSION}");
            0
        }
        Command::Build {
            input,
            output,
            emit_ir,
            release,
            no_opt,
        } => {
            let loaded = match super::modules::load(&input) {
                Ok(loaded) => loaded,
                Err(error) => {
                    eprintln!("navascript: {error}");
                    return 1;
                }
            };
            let source = loaded.source;
            let parsed = navascript_parser::parse(&source);
            if parsed.diagnostics.has_errors() {
                eprint!("{}", parsed.diagnostics.render(&source));
                return 1;
            }
            let checked = navascript_typechecker::check(&parsed.program, &source);
            if checked.diagnostics.has_errors() {
                eprint!("{}", checked.diagnostics.render(&source));
                return 1;
            }
            let lowered = navascript_ir::lower(&parsed.program, &checked.typed, &source);
            if lowered.diagnostics.has_errors() {
                eprint!("{}", lowered.diagnostics.render(&source));
                return 1;
            }
            let level = if no_opt {
                navascript_optimizer::Level::None
            } else if release {
                navascript_optimizer::Level::Aggressive
            } else {
                navascript_optimizer::Level::Minimal
            };
            let optimized = navascript_optimizer::optimize(&lowered.module, level);
            let triple = match super::build::host_triple() {
                Ok(triple) => triple,
                Err(error) => {
                    eprintln!("navascript: {error}");
                    return 1;
                }
            };
            let llvm_ir = match navascript_codegen::generate(&optimized.module, &triple) {
                Ok(ir) => ir,
                Err(error) => {
                    eprintln!("navascript: code generation failed: {error}");
                    return 1;
                }
            };
            if emit_ir {
                if release && !no_opt {
                    match super::build::optimize_llvm_ir(&llvm_ir) {
                        Ok(optimized_ir) => print!("{optimized_ir}"),
                        Err(error) => {
                            eprintln!("navascript: LLVM optimization failed: {error}");
                            return 1;
                        }
                    }
                } else {
                    print!("{llvm_ir}");
                }
                return 0;
            }

            let output = output.unwrap_or_else(|| {
                PathBuf::from(
                    input
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("a.out"),
                )
            });
            if let Err(error) = super::build::emit_native(
                &llvm_ir,
                navascript_codegen::RUNTIME_FILES,
                &output,
                release && !no_opt,
            ) {
                eprintln!("navascript: native build failed: {error}");
                return 1;
            }
            println!("navascript: built {}", output.display());
            0
        }
        Command::Error(message) => {
            eprintln!("{message}");
            eprintln!("Run 'navascript --help' for usage.");
            2
        }
    }
}
