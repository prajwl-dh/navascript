//! NavaScript IR optimizer crate.
//!
//! The public seam is [`optimize`]: an IR module plus a level in, an equivalent
//! module plus pass statistics out. Passes live in private modules behind that
//! seam (ADR-0016), and the result is always run through the IR verifier before
//! it is returned; an invalid optimization silently falls back to the input.

mod dce;
mod escape;
mod fold;
mod inline;
mod utils;

use navascript_ir::{IrModule, verify};

/// How much optimization to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Pass the module through unchanged (the benchmark baseline).
    None,
    /// Constant folding and dead code elimination.
    Minimal,
    /// Minimal passes plus function inlining.
    Aggressive,
}

/// How many transformations the passes applied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub folded: usize,
    pub eliminated: usize,
    pub inlined: usize,
    pub promoted: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizeOutput {
    pub module: IrModule,
    pub stats: Stats,
}

/// Optimize a verified IR module, preserving its observable behaviour.
pub fn optimize(module: &IrModule, level: Level) -> OptimizeOutput {
    if level == Level::None {
        return OptimizeOutput {
            module: module.clone(),
            stats: Stats::default(),
        };
    }

    let mut stats = Stats::default();
    let mut current = simplify(module, &mut stats);
    if level == Level::Aggressive {
        let (inlined, count) = inline::run(&current);
        stats.inlined = count;
        let (escaped, promoted) = escape::run(&inlined);
        stats.promoted = promoted;
        current = simplify(&escaped, &mut stats);
    }

    if !verify(&current).is_empty() {
        return OptimizeOutput {
            module: module.clone(),
            stats: Stats::default(),
        };
    }
    OptimizeOutput {
        module: current,
        stats,
    }
}

/// Run folding and dead-code elimination until neither changes the module.
fn simplify(module: &IrModule, stats: &mut Stats) -> IrModule {
    let mut current = module.clone();
    for _ in 0..32 {
        let (folded, folded_count) = fold::run(&current);
        stats.folded += folded_count;
        let (dced, eliminated_count) = dce::run(&folded);
        stats.eliminated += eliminated_count;
        if dced == current {
            return dced;
        }
        current = dced;
    }
    current
}
