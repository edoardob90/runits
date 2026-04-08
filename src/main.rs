//! `runits` CLI entry point.
//!
//! Dispatches to one-shot conversion, REPL, batch mode, or subcommands
//! based on CLI arguments. Loads optional config from
//! `~/.config/runits/config.toml` and merges with CLI flags.

use std::io::IsTerminal;

use clap::Parser;
use runits::{
    cli::Cli,
    cli::Commands,
    config::Config,
    convert, database,
    error::RUnitsError,
    format::{self, FormatOptions},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), RUnitsError> {
    let cli = Cli::parse();

    // Subcommands take priority (no config needed).
    if let Some(cmd) = &cli.command {
        return match cmd {
            Commands::Completions { shell } => {
                generate_completions(*shell);
                Ok(())
            }
        };
    }

    let config = Config::load();

    // Dispatch based on positional args.
    match (&cli.quantity, &cli.target) {
        (Some(quantity), Some(target)) => run_oneshot(&cli, &config, quantity, target),
        (None, None) if cli.batch => run_batch(&cli, &config),
        (None, None) => run_repl(&cli, &config),
        _ => {
            eprintln!("Usage: runits <quantity> <target>, or runits (no args) for REPL mode");
            std::process::exit(2);
        }
    }
}

/// Build FormatOptions by merging CLI flags over config defaults.
fn resolve_opts(cli: &Cli, config: &Config, is_repl: bool) -> FormatOptions {
    let is_tty = std::io::stdout().is_terminal();
    FormatOptions {
        // CLI --precision overrides config precision; both override default 6.
        precision: cli.precision.or(config.precision),
        scientific: cli.scientific,
        to_base: cli.to_base,
        color: format::should_color(config.color.unwrap_or(is_tty)),
        unicode: cli.pretty || config.unicode.unwrap_or(is_repl && is_tty),
        annotations: is_repl,
        json: cli.json,
    }
}

fn run_oneshot(
    cli: &Cli,
    config: &Config,
    quantity: &str,
    target: &str,
) -> Result<(), RUnitsError> {
    let db = database::global();
    let conv = convert::run_conversion(quantity, target, db)?;
    let opts = resolve_opts(cli, config, false);
    println!("{}", format::format_result(&conv, &opts));
    Ok(())
}

fn run_repl(cli: &Cli, config: &Config) -> Result<(), RUnitsError> {
    let opts = resolve_opts(cli, config, true);
    runits::repl::run(&opts);
    Ok(())
}

fn run_batch(cli: &Cli, config: &Config) -> Result<(), RUnitsError> {
    let db = database::global();
    let opts = resolve_opts(cli, config, false);

    let stdin = std::io::stdin();
    for line in std::io::BufRead::lines(stdin.lock()) {
        let line = line.expect("failed to read stdin line");
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match runits::repl::parse_repl_line(line) {
            Some((source, target)) => match convert::run_conversion(source, target, db) {
                Ok(result) => println!("{}", format::format_result(&result, &opts)),
                Err(e) => eprintln!("Error: {e}"),
            },
            None => eprintln!("malformed line: {line}"),
        }
    }
    Ok(())
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "runits", &mut std::io::stdout());
}
