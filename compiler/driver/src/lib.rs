//! Nava compiler driver.
//!
//! This crate contains the command-line interface and, in later phases, the
//! orchestration of the compiler pipeline. Front-end and back-end code live in
//! their own workspace crates under `compiler/`.

pub mod build;
pub mod cli;
pub mod modules;
