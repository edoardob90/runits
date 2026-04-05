//! clap-derived CLI argument layout for Phase 2.
//!
//! Two positional args: a quantity (number + source unit) and a target unit.
//! Doc comments on each field become clap's help text — keep them one line
//! each and user-facing. Phase 3+ will add flags here (--precision,
//! --scientific, --verbose, subcommands for REPL / constants / etc.).

use clap::Parser;

/// Convert quantities between units.
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    /// Quantity to convert, with source unit. Example: "10 ft"
    pub quantity: String,

    /// Target unit. Example: "m"
    pub target: String,
}
