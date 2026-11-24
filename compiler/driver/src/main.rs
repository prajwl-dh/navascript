//! Entry point for the `navascript` binary.

fn main() {
    std::process::exit(navascript_driver::cli::run());
}
