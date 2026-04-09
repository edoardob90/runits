//! clap-derived CLI argument layout.
//!
//! Supports one-shot conversion (`runits "10 ft" "m"`), REPL mode
//! (`runits` with no args), batch mode (`--batch`), and subcommands.

use clap::{Parser, Subcommand, ValueEnum};

/// Convert quantities between units.
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Cli {
    /// Quantity to convert (omit for REPL mode). Example: "10 ft"
    pub quantity: Option<String>,

    /// Target unit. Example: "m"
    pub target: Option<String>,

    /// Number of significant figures in output
    #[arg(short, long)]
    pub precision: Option<usize>,

    /// Force scientific notation in output
    #[arg(short, long)]
    pub scientific: bool,

    /// Expand result unit to base SI components (e.g., newton → kg*m/s^2)
    #[arg(long)]
    pub to_base: bool,

    /// Use Unicode symbols in output (kg·m/s² instead of kg*m/s^2)
    #[arg(long)]
    pub pretty: bool,

    /// Output result as JSON
    #[arg(long)]
    pub json: bool,

    /// Read conversions from stdin, one per line
    #[arg(long)]
    pub batch: bool,

    /// REPL intro banner mode
    #[arg(long, value_enum)]
    pub intro_banner: Option<BannerMode>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BannerMode {
    Long,
    Short,
    Off,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// List known units, optionally filtered by physical quantity
    #[command(name = "list-units")]
    ListUnits {
        /// Filter by quantity name (e.g., "velocity", "force", "length")
        filter: Option<String>,
    },
}
