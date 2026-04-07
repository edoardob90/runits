//! clap-derived CLI argument layout.
//!
//! Two positional args (quantity, target) plus formatting flags.

use clap::Parser;

/// Convert quantities between units.
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    /// Quantity to convert, with source unit. Example: "10 ft"
    pub quantity: String,

    /// Target unit. Example: "m"
    pub target: String,

    /// Number of significant figures in output (default: 6)
    #[arg(short, long)]
    pub precision: Option<usize>,

    /// Force scientific notation in output
    #[arg(short, long)]
    pub scientific: bool,

    /// Expand result unit to base SI components (e.g., newton → kg*m/s^2)
    #[arg(long)]
    pub to_base: bool,
}
