//! Entry point for the `nava` binary.

fn main() {
    std::process::exit(nava_driver::cli::run());
}
