//! `runits` CLI entry point.
//!
//! Dispatches to one-shot conversion, REPL, batch mode, or subcommands
//! based on CLI arguments. Loads optional config from
//! `~/.config/runits/config.toml` and merges with CLI flags.

use std::io::IsTerminal;

use clap::Parser;
use runits::{
    cli::{Cli, Commands, ListWhat},
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
        let config = Config::load();
        let opts = resolve_opts(&cli, &config, false);
        return match cmd {
            Commands::Completions { shell } => {
                generate_completions(*shell);
                Ok(())
            }
            Commands::List { what } => {
                run_list(what, &opts);
                Ok(())
            }
        };
    }

    let config = Config::load();

    // --info flag: print database/config info and exit.
    if cli.info {
        let opts = resolve_opts(&cli, &config, false);
        runits::repl::print_info_standalone(&opts);
        return Ok(());
    }

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
        explain: cli.explain,
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
    let banner = cli.intro_banner.unwrap_or_else(|| {
        // Config override, or default to long on TTY.
        match config.intro_banner.as_deref() {
            Some("off") => runits::cli::BannerMode::Off,
            Some("short") => runits::cli::BannerMode::Short,
            _ => {
                if std::io::stdout().is_terminal() {
                    runits::cli::BannerMode::Long
                } else {
                    runits::cli::BannerMode::Off
                }
            }
        }
    });
    runits::repl::run(&opts, banner);
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

fn run_list(what: &ListWhat, opts: &FormatOptions) {
    let db = database::global();

    match what {
        ListWhat::Units { filter } => list_units(filter.as_deref(), db, opts),
        ListWhat::Dimensions | ListWhat::Quantities => list_dimensions(opts),
        ListWhat::Constants => list_constants(opts),
    }
}

fn list_units(filter: Option<&str>, db: &runits::database::UnitDatabase, opts: &FormatOptions) {
    if let Some(query) = filter {
        // Filtered: try as quantity name (exact, then prefix), then as unit name.
        let dims = runits::annotations::dimensions_for_name(query).or_else(|| {
            let q = query.to_lowercase();
            runits::annotations::all_quantity_names()
                .into_iter()
                .find(|n| n.to_lowercase().starts_with(&q))
                .and_then(runits::annotations::dimensions_for_name)
        });

        if let Some(dims) = dims {
            let qty_name = runits::annotations::quantity_name(&dims).unwrap_or(query);
            let dims_vec: Vec<_> = dims.iter().map(|(d, &e)| (d.clone(), e)).collect();
            let synthetic = runits::Unit::new("_query", 1.0, &dims_vec);
            let compat = db.compatible_units(&synthetic);
            if opts.json {
                print_json_unit_list(qty_name, &compat);
            } else {
                println!(
                    "{}",
                    format::format_unit_list(qty_name, &compat, Some(&dims), opts)
                );
            }
        } else if let Ok(unit) = runits::parser::parse_unit_name(query, db) {
            let qty_name =
                runits::annotations::quantity_name(&unit.dimensions).unwrap_or("(unnamed)");
            let compat = db.compatible_units(&unit);
            if opts.json {
                print_json_unit_list(qty_name, &compat);
            } else {
                println!(
                    "{}",
                    format::format_unit_list(qty_name, &compat, Some(&unit.dimensions), opts)
                );
            }
        } else {
            eprintln!("Error: unknown dimension or unit: '{query}'");
            let names = runits::annotations::all_quantity_names();
            eprintln!("  Known dimensions: {}", names.join(", "));
            std::process::exit(1);
        }
    } else {
        let groups = runits::repl::build_unit_groups(db);
        if opts.json {
            print_json_all_groups(&groups);
        } else {
            println!("{}", format::format_all_units_grouped(&groups, opts));
        }
    }
}

fn list_dimensions(opts: &FormatOptions) {
    let names = runits::annotations::all_quantity_names();
    if opts.json {
        let json: Vec<String> = names.iter().map(|n| format!("\"{}\"", n)).collect();
        println!("[{}]", json.join(","));
    } else {
        let t = runits::theme::Theme::new(opts.color);
        for name in &names {
            let colored = runits::annotations::dimensions_for_name(name)
                .map(|d| t.paint(name, t.dims_style(&d)))
                .unwrap_or_else(|| name.to_string());
            println!("  {}", colored);
        }
    }
}

fn list_constants(opts: &FormatOptions) {
    let const_db = runits::database::constants::global();
    let mut constants = const_db.all_unique();
    constants.sort_by_key(|c| c.name);
    if opts.json {
        let entries: Vec<String> = constants
            .iter()
            .map(|c| {
                format!(
                    "{{\"name\":\"{}\",\"value\":{},\"unit\":\"{}\"}}",
                    c.name, c.value, c.unit.name
                )
            })
            .collect();
        println!("[{}]", entries.join(","));
    } else {
        let t = runits::theme::Theme::new(opts.color);
        for c in &constants {
            let val = runits::units::quantity::format_value(c.value, 6, false);
            println!("  {} = {} {}", t.cst(c.name), t.num(&val), c.unit.name);
        }
    }
}

fn print_json_unit_list(quantity: &str, units: &[String]) {
    println!(
        "{{\"quantity\":\"{}\",\"units\":[{}]}}",
        quantity,
        units
            .iter()
            .map(|u| format!("\"{}\"", u))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn print_json_all_groups(groups: &[(String, Vec<String>)]) {
    let entries: Vec<String> = groups
        .iter()
        .map(|(qty, units)| {
            let units_json = units
                .iter()
                .map(|u| format!("\"{}\"", u))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{\"quantity\":\"{}\",\"units\":[{}]}}", qty, units_json)
        })
        .collect();
    println!("[{}]", entries.join(","));
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "runits", &mut std::io::stdout());
}
