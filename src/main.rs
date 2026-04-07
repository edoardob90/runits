//! `runits` CLI entry point.
//!
//! Parses positional args (`quantity`, `target`) plus optional formatting
//! flags, performs a dimensional-safe conversion, and prints the result.

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

    let has_explicit_flags = cli.precision.is_some() || cli.scientific || cli.to_base;

    if has_explicit_flags {
        let sig_figs = cli.precision.unwrap_or(6);
        let unit_name = if cli.to_base {
            result.unit.to_base_unit_string()
        } else {
            result.unit.name.clone()
        };
        println!(
            "{}",
            result.format_with(sig_figs, cli.scientific, &unit_name)
        );
    } else {
        println!("{}", result);
    }
    Ok(())
}
