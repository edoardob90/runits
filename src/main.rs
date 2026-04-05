//! `runits` CLI entry point.
//!
//! Parses two positional args (`quantity`, `target`), looks up the source
//! unit from the embedded database, performs a dimensional-safe conversion,
//! and prints the result. Errors surface as plain single-line messages on
//! stderr and exit with code 1; clap handles its own errors (bad usage,
//! `--help`, `--version`) with its own exit codes.

use clap::Parser;
use runits::{cli::Cli, database, error::RUnitsError, parser};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), RUnitsError> {
    let cli = Cli::parse();
    let db = database::global();

    let source = parser::parse_quantity(&cli.quantity, db)?;
    let target = parser::parse_unit_name(&cli.target, db)?;
    let result = source.convert_to(&target)?;

    println!("{}", result);
    Ok(())
}
