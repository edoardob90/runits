//! Interactive REPL mode.
//!
//! Entered when `runits` is called with no arguments. Each line is a
//! conversion query in the form `<quantity> -> <target>` (also accepts
//! `to`, `in`, `as` delimiters). History is persisted at
//! `~/.config/runits/history`.

mod helper;

use crate::annotations::{self, quantity_name};
use crate::database::constants;
use crate::database::{self, UnitDatabase};
use crate::format::{self, FormatOptions};
use crate::parser;
use crate::theme::Theme;
use crate::{Dimension, convert};
use helper::UnitsHelper;
use rustyline::Editor;
use rustyline::error::ReadlineError;
use std::path::PathBuf;

/// Parse a REPL line into (source, target) by splitting on a delimiter.
///
/// Tries delimiters in order: ` -> `, ` to `, ` in `, ` as `.
pub fn parse_repl_line(line: &str) -> Option<(&str, &str)> {
    for delim in [" -> ", " to ", " in ", " as "] {
        if let Some(pos) = line.find(delim) {
            let source = line[..pos].trim();
            let target = line[pos + delim.len()..].trim();
            if !source.is_empty() && !target.is_empty() {
                return Some((source, target));
            }
        }
    }
    None
}

/// Run the interactive REPL loop.
pub fn run(opts: &FormatOptions, banner: crate::cli::BannerMode) {
    let mut rl = Editor::new().expect("failed to initialize line editor");
    rl.set_helper(Some(UnitsHelper {
        db: database::global(),
    }));

    let history_path = config_dir().map(|d| d.join("history"));
    if let Some(ref path) = history_path {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        let _ = rl.load_history(path);
    }

    let t = Theme::new(opts.color);
    let db = database::global();
    let mut last_conversion: Option<convert::ConversionResult> = None;

    print_banner(banner, &t, db);

    loop {
        match rl.readline(">>> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line == "quit" || line == "exit" {
                    break;
                }
                // `?` stays a strict equality check — it's punctuation, not a word.
                if line == "?" {
                    print_help(&t);
                    continue;
                }

                // Argless commands: info, help, explain. Reject trailing args
                // uniformly via `print_extra_args_error`.
                if let Some(args) = match_command(line, "info") {
                    if !args.is_empty() {
                        print_extra_args_error("info", args, &t);
                    } else {
                        print_info(&t, db);
                    }
                    continue;
                }
                if let Some(args) = match_command(line, "help") {
                    if !args.is_empty() {
                        print_extra_args_error("help", args, &t);
                    } else {
                        print_help(&t);
                    }
                    continue;
                }
                if let Some(args) = match_command(line, "explain") {
                    let _ = rl.add_history_entry(line);
                    if !args.is_empty() {
                        print_extra_args_error("explain", args, &t);
                    } else if let Some(ref conv) = last_conversion {
                        let explain_opts = FormatOptions {
                            explain: true,
                            ..opts.clone()
                        };
                        println!("{}", format::format_explain(conv, &explain_opts));
                    } else {
                        eprintln!(
                            "{}",
                            t.err("No conversion to explain. Run a conversion first.")
                        );
                    }
                    continue;
                }

                // Arged commands: const, list, search. Reject empty args.
                if let Some(args) = match_command(line, "const") {
                    let _ = rl.add_history_entry(line);
                    if args.is_empty() {
                        print_missing_arg_error("const", "const <name>", &t);
                    } else {
                        handle_const_command(args, opts);
                    }
                    continue;
                }
                if let Some(args) = match_command(line, "list") {
                    let _ = rl.add_history_entry(line);
                    if args.is_empty() {
                        print_missing_arg_error(
                            "list",
                            "list units|dimensions|constants [filter]",
                            &t,
                        );
                    } else {
                        handle_list_command(args, db, opts);
                    }
                    continue;
                }
                // Legacy alias: `search` → `list units`
                if let Some(args) = match_command(line, "search") {
                    let _ = rl.add_history_entry(line);
                    if args.is_empty() {
                        print_missing_arg_error("search", "search <query>", &t);
                    } else {
                        handle_list_command(&format!("units {args}"), db, opts);
                    }
                    continue;
                }

                let _ = rl.add_history_entry(line);

                if let Some(conv) = handle_input(line, db, opts) {
                    last_conversion = Some(conv);
                }
            }
            Err(ReadlineError::Interrupted) => continue, // Ctrl-C: clear line
            Err(ReadlineError::Eof) => break,            // Ctrl-D: exit
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }

    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }
}

/// Main REPL dispatch: route input to the right handler.
fn print_banner(mode: crate::cli::BannerMode, t: &Theme, db: &UnitDatabase) {
    use crate::cli::BannerMode;
    match mode {
        BannerMode::Off => {}
        BannerMode::Short => {
            println!(
                "{} {} — {}, {} units. Type {} or {} to exit.",
                t.kw("runits"),
                env!("CARGO_PKG_VERSION"),
                format::UNIT_SYSTEM,
                db.len(),
                t.kw("quit"),
                t.kw("Ctrl-D"),
            );
        }
        BannerMode::Long => {
            println!();
            println!("  {} {}", t.kw("runits"), env!("CARGO_PKG_VERSION"));
            println!("  Unit converter with dimensional analysis");
            println!();
            println!("  {} {}", t.dim("Unit system:"), format::UNIT_SYSTEM,);
            println!(
                "  {} {} (builtin) + SI/binary prefixes",
                t.dim("Database:"),
                db.len(),
            );
            let config_path = config_dir()
                .map(|d| d.join("config.toml"))
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "not found".to_string());
            println!("  {} {}", t.dim("Config:"), config_path);
            println!();
            println!(
                "  Syntax: {} {} {}",
                t.dim("<quantity>"),
                t.kw("->"),
                t.dim("<target>"),
            );
            println!(
                "  Type {} for unit help, {} for status, {} to exit.",
                t.kw("?"),
                t.kw("info"),
                t.kw("quit"),
            );
            println!();
        }
    }
}

/// Print database/config info for CLI `--info` flag (no REPL context).
pub fn print_info_standalone(opts: &FormatOptions) {
    let t = Theme::new(opts.color);
    let db = database::global();
    print_info(&t, db);
}

fn print_help(t: &Theme) {
    // (visible_command, styled_command, description)
    // (plain_text for width calc, styled_text, description)
    let rows: Vec<(&str, String, &str)> = vec![
        (
            "<qty> to <unit>",
            format!("{} {} {}", t.dim("<qty>"), t.kw("to"), t.dim("<unit>")),
            "convert between units",
        ),
        (
            "? <name>",
            format!("{} {}", t.kw("?"), t.dim("<name>")),
            "unit or constant info",
        ),
        (
            "list units|dimensions|constants [filter]",
            format!(
                "{} {} {}",
                t.kw("list"),
                t.dim("units|dimensions|constants"),
                t.dim("[filter]")
            ),
            "browse the database",
        ),
        (
            "const <name>",
            format!("{} {}", t.kw("const"), t.dim("<name>")),
            "show constant value",
        ),
        (
            "explain",
            t.kw("explain"),
            "show last conversion step-by-step",
        ),
        ("info", t.kw("info"), "database & config info"),
        ("help", t.kw("help"), "this help"),
        ("quit", t.kw("quit"), "exit"),
    ];

    let col_width = rows
        .iter()
        .map(|(plain, _, _)| plain.len())
        .max()
        .unwrap_or(0)
        + 2;

    println!("  {}", t.kw("Commands:"));
    for (plain, styled, desc) in &rows {
        let pad = col_width.saturating_sub(plain.len());
        println!("    {styled}{:pad$}  {desc}", "");
    }
}

fn print_info(t: &Theme, db: &UnitDatabase) {
    // Unit system and database
    println!("  {} {}", t.dim("Unit system:"), format::UNIT_SYSTEM,);
    println!(
        "  {} {} (builtin) + SI/binary prefixes",
        t.dim("Database:"),
        db.len(),
    );
    // Config file
    let config_path = config_dir()
        .map(|d| d.join("config.toml"))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "not found".to_string());
    println!("  {} {}", t.dim("Config:"), config_path);
    // History file
    let history_path = config_dir()
        .map(|d| d.join("history"))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "not found".to_string());
    println!("  {} {}", t.dim("History:"), history_path);
    // FUTURE(theme-config): theme name should come from config when
    // user-defined themes (config.toml [theme] section) are supported.
    print!("  {}", t.dim("Theme:"));
    let theme_legend: Vec<String> = Dimension::ALL
        .iter()
        .map(|dim| t.paint(dim.name(), t.dimension_style(dim)))
        .collect();
    println!(" {}", theme_legend.join(" · "));
    // Quantities
    println!(
        "  {} {} quantities registered",
        t.dim("Annotations:"),
        crate::annotations::quantity_name_count(),
    );
}

/// Dispatch a REPL line: conversion, help query, or bare quantity.
///
/// Returns `Some(result)` for successful conversions (used by `explain`
/// to store the last conversion). Returns `None` for help queries,
/// bare echoes, and errors.
fn handle_input(
    line: &str,
    db: &UnitDatabase,
    opts: &FormatOptions,
) -> Option<convert::ConversionResult> {
    // 1. Delimiter-based input (check first, so "100 km/h -> ?" is caught).
    if let Some((source, target)) = parse_repl_line(line) {
        if target == "?" {
            handle_quantity_help(source, db, opts);
            return None;
        }
        match convert::run_conversion(source, target, db) {
            Ok(result) => {
                println!("{}", format::format_result(&result, opts));
                return Some(result);
            }
            Err(_) => {
                // Fallback: try source as a constant name (e.g., "c_0 to mph").
                if let Some(result) = try_constant_conversion(source, target, db) {
                    match result {
                        Ok(r) => {
                            println!("{}", format::format_result(&r, opts));
                            return Some(r);
                        }
                        Err(e) => print_error(&e, opts),
                    }
                } else {
                    // Re-run to get the original error message.
                    if let Err(e) = convert::run_conversion(source, target, db) {
                        print_error(&e, opts);
                    }
                }
            }
        }
        return None;
    }

    // 2. ? prefix/suffix: "? meter", "meter ?", "1 N ?"
    //    If query contains a digit → it's a quantity ("1 N ?", "2.5 ft ?").
    //    Otherwise → unit help ("? meter", "N ?").
    if let Some(query) = strip_question_mark(line) {
        let has_number = query.contains(|c: char| c.is_ascii_digit());
        if has_number && let Ok(qty) = parser::parse_quantity(query, db) {
            handle_quantity_help_from_qty(qty, db, opts);
            return None;
        }
        handle_unit_help(query, db, opts);
        return None;
    }

    // 3. No delimiter, no ? — try parsing as a bare quantity.
    //    Falls back to constants DB if quantity parsing fails.
    match parser::parse_quantity(line, db) {
        Ok(qty) => {
            let annotation = quantity_name(&qty.unit.dimensions);
            let result = convert::ConversionResult {
                source: qty.clone(),
                result: qty,
                annotation,
            };
            println!("{}", format::format_result(&result, opts));
        }
        Err(e) => {
            // Before reporting error, check if the whole input is a constant name.
            let const_db = constants::global();
            if let Some(c) = const_db.lookup(line) {
                let qty = crate::units::Quantity::new(c.value, c.unit.clone());
                let annotation = quantity_name(&qty.unit.dimensions);
                let result = convert::ConversionResult {
                    source: qty.clone(),
                    result: qty,
                    annotation,
                };
                println!("{}", format::format_result(&result, opts));
            } else {
                print_error(&e, opts);
            }
        }
    }
    None
}

/// Match a REPL command at the start of a line, returning its argument (possibly empty).
///
/// Returns `Some("")` if the line is exactly the command name, `Some(args)` if
/// there's whitespace + trailing args, or `None` if the command doesn't match
/// at all. This lets the caller decide how to handle empty vs non-empty args
/// (some commands require args, others reject them).
fn match_command<'a>(line: &'a str, command: &str) -> Option<&'a str> {
    if line == command {
        return Some("");
    }
    let rest = line.strip_prefix(command)?;
    if rest.starts_with(' ') || rest.starts_with('\t') {
        Some(rest.trim())
    } else {
        None
    }
}

/// Print a consistent "command takes no arguments" error for argless commands.
fn print_extra_args_error(command: &str, args: &str, t: &Theme) {
    eprintln!(
        "{}",
        t.err(&format!(
            "Error: '{command}' takes no arguments (got '{args}')"
        ))
    );
    eprintln!("  Usage: {command}");
}

/// Print a consistent "command requires an argument" error for arged commands.
fn print_missing_arg_error(command: &str, usage: &str, t: &Theme) {
    eprintln!(
        "{}",
        t.err(&format!("Error: '{command}' requires an argument"))
    );
    eprintln!("  Usage: {usage}");
}

/// Strip `?` from start or end, returning the remainder to look up.
fn strip_question_mark(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix('?') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(rest);
        }
    }
    if let Some(rest) = line.strip_suffix('?') {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(rest);
        }
    }
    None
}

/// Handle `? meter` or `meter ?` — show unit info + compatible units.
///
/// Falls back to the constants database if the query doesn't match any unit.
fn handle_unit_help(query: &str, db: &UnitDatabase, opts: &FormatOptions) {
    match parser::parse_unit_name(query, db) {
        Ok(unit) => {
            let aliases = db.aliases_for(&unit.name);
            let compatible = db.compatible_units(&unit);
            let annotation = quantity_name(&unit.dimensions);
            println!(
                "{}",
                format::format_unit_info(&unit, &aliases, &compatible, annotation, opts)
            );
        }
        Err(e) => {
            // Fall back to constants database before reporting error.
            let const_db = constants::global();
            if let Some(c) = const_db.lookup(query) {
                println!("{}", format::format_constant_info(c, opts));
            } else {
                print_error(&e, opts);
            }
        }
    }
}

/// Handle `100 km/h -> ?` — parse source, then show help.
fn handle_quantity_help(source: &str, db: &UnitDatabase, opts: &FormatOptions) {
    match parser::parse_quantity(source, db) {
        Ok(qty) => handle_quantity_help_from_qty(qty, db, opts),
        Err(e) => print_error(&e, opts),
    }
}

/// Echo a quantity with annotation + list compatible units.
fn handle_quantity_help_from_qty(
    qty: crate::units::Quantity,
    db: &UnitDatabase,
    opts: &FormatOptions,
) {
    let annotation = quantity_name(&qty.unit.dimensions);
    let result = convert::ConversionResult {
        source: qty.clone(),
        result: qty.clone(),
        annotation,
    };
    println!("{}", format::format_result(&result, opts));

    let t = Theme::new(opts.color);
    let compatible = db.compatible_units(&qty.unit);
    if compatible.is_empty() {
        println!("  {}", t.dim("No other compatible units in database."));
    } else {
        let style = t.unit_style(&qty.unit);
        let list = compatible
            .iter()
            .map(|u| t.paint(u, style))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  Compatible: {}", list);
    }
}

/// Try interpreting `source` as a constant name and converting to `target`.
///
/// Returns `None` if source is not a known constant, `Some(result)` otherwise.
fn try_constant_conversion(
    source: &str,
    target: &str,
    db: &UnitDatabase,
) -> Option<Result<convert::ConversionResult, crate::error::RUnitsError>> {
    let const_db = constants::global();
    let c = const_db.lookup(source.trim())?;
    let qty = crate::units::Quantity::new(c.value, c.unit.clone());
    let target_unit = match parser::parse_unit_name(target, db) {
        Ok(u) => u,
        Err(e) => return Some(Err(e)),
    };
    let result = match qty.convert_to(&target_unit) {
        Ok(r) => r,
        Err(e) => return Some(Err(e)),
    };
    let annotation = quantity_name(&result.unit.dimensions);
    Some(Ok(convert::ConversionResult {
        source: qty,
        result,
        annotation,
    }))
}

/// Handle `list <what> [filter]` — list units, dimensions, or constants.
///
/// Accepts unambiguous prefixes: `list u` → units, `list d` → dimensions, etc.
/// Matches the tab-completion behavior in `helper.rs`, so users who hit Enter
/// before Tab get the same resolution.
fn handle_list_command(rest: &str, db: &UnitDatabase, opts: &FormatOptions) {
    let t = Theme::new(opts.color);
    let (what, filter) = match rest.split_once(|c: char| c.is_whitespace()) {
        Some((w, f)) => (w.trim(), Some(f.trim())),
        None => (rest.trim(), None),
    };

    let Some(canonical) = resolve_list_subcommand(what) else {
        eprintln!(
            "{}",
            t.err(&format!("Error: unknown list target: '{what}'"))
        );
        eprintln!("  Usage: list units|dimensions|constants [filter]");
        return;
    };

    match canonical {
        "units" => handle_list_units(filter, db, opts),
        "dimensions" => handle_list_dimensions(opts),
        "constants" => handle_list_constants(opts),
        _ => unreachable!("resolve_list_subcommand only returns known canonicals"),
    }
}

/// Resolve a (possibly abbreviated) `list` subcommand to its canonical form.
///
/// Case-insensitive. Accepts exact matches and unambiguous prefixes — the four
/// valid inputs (units, dimensions, constants, quantities) have distinct first
/// letters, so single-letter prefixes are unambiguous. `quantities` is an alias
/// for `dimensions` (same listing).
fn resolve_list_subcommand(what: &str) -> Option<&'static str> {
    // (input, canonical-returned). Aliases map to the same canonical.
    const ENTRIES: &[(&str, &str)] = &[
        ("units", "units"),
        ("dimensions", "dimensions"),
        ("quantities", "dimensions"),
        ("constants", "constants"),
    ];
    let lower = what.to_lowercase();
    if lower.is_empty() {
        return None;
    }

    // Exact match wins over prefix (so "constant" doesn't become ambiguous
    // just because there are multiple words in the table).
    if let Some(&(_, canonical)) = ENTRIES.iter().find(|(input, _)| *input == lower) {
        return Some(canonical);
    }

    // Prefix match: must be unique across distinct canonicals.
    let matches: Vec<&'static str> = ENTRIES
        .iter()
        .filter(|(input, _)| input.starts_with(&lower))
        .map(|(_, canonical)| *canonical)
        .collect();

    // Collapse duplicates (e.g., if both "quantities" and "questions" existed
    // and shared a canonical — not today, but defensive). Using a dedup pass
    // rather than a HashSet to keep this alloc-free at call time.
    let first = matches.first()?;
    if matches.iter().all(|m| m == first) {
        Some(*first)
    } else {
        None
    }
}

/// List units, optionally filtered by dimension/quantity name.
fn handle_list_units(filter: Option<&str>, db: &UnitDatabase, opts: &FormatOptions) {
    let t = Theme::new(opts.color);

    let Some(query) = filter else {
        // No filter — show all units grouped by dimension.
        let groups = build_unit_groups(db);
        for (qty_name, unit_names) in &groups {
            let dims = annotations::dimensions_for_name(qty_name);
            println!(
                "{}",
                format::format_unit_list(qty_name, unit_names, dims.as_ref(), opts)
            );
        }
        return;
    };

    // Try exact quantity name match (case-insensitive).
    if let Some(dims) = annotations::dimensions_for_name(query) {
        print_unit_search_results(query, &dims, db, opts);
        return;
    }

    // Try case-insensitive prefix match on quantity names.
    let query_lower = query.to_lowercase();
    let prefix_match = annotations::all_quantity_names()
        .into_iter()
        .find(|name| name.to_lowercase().starts_with(&query_lower));
    if let Some(matched_name) = prefix_match
        && let Some(dims) = annotations::dimensions_for_name(matched_name)
    {
        print_unit_search_results(matched_name, &dims, db, opts);
        return;
    }

    // Try as unit name (e.g., "meter", "N").
    if let Ok(unit) = parser::parse_unit_name(query, db) {
        let compat = db.compatible_units(&unit);
        let qty_name = quantity_name(&unit.dimensions).unwrap_or("(unnamed)");
        println!(
            "{}",
            format::format_unit_list(qty_name, &compat, Some(&unit.dimensions), opts)
        );
        return;
    }

    // Nothing matched.
    eprintln!(
        "{}",
        t.err(&format!("Error: unknown dimension or unit: '{query}'"))
    );
    print_known_dimensions(&t);
}

fn handle_list_dimensions(opts: &FormatOptions) {
    let t = Theme::new(opts.color);
    let names = annotations::all_quantity_names();
    for name in &names {
        let colored = annotations::dimensions_for_name(name)
            .map(|d| t.paint(name, t.dims_style(&d)))
            .unwrap_or_else(|| name.to_string());
        println!("  {colored}");
    }
}

fn handle_list_constants(opts: &FormatOptions) {
    let t = Theme::new(opts.color);
    let const_db = constants::global();
    let mut all = const_db.all_unique();
    all.sort_by_key(|c| c.name);
    for c in &all {
        let val = crate::units::quantity::format_value(c.value, 6, false);
        println!("  {} = {} {}", t.cst(c.name), t.num(&val), c.unit.name);
    }
}

/// Print units matching a dimension query.
fn print_unit_search_results(
    query: &str,
    dims: &crate::units::dimension::DimensionMap,
    db: &UnitDatabase,
    opts: &FormatOptions,
) {
    let qty_name = quantity_name(dims).unwrap_or(query);
    let dims_vec: Vec<_> = dims.iter().map(|(d, &e)| (d.clone(), e)).collect();
    let synthetic = crate::units::Unit::new("_query", 1.0, &dims_vec);
    let compat = db.compatible_units(&synthetic);
    println!(
        "{}",
        format::format_unit_list(qty_name, &compat, Some(dims), opts)
    );
}

/// Print the colored list of known dimensions/quantities.
fn print_known_dimensions(t: &Theme) {
    let names = annotations::all_quantity_names();
    let colored_names: Vec<String> = names
        .iter()
        .map(|name| {
            annotations::dimensions_for_name(name)
                .map(|d| t.paint(name, t.dims_style(&d)))
                .unwrap_or_else(|| name.to_string())
        })
        .collect();
    eprintln!("  Known dimensions: {}", colored_names.join(", "));
}

/// Build groups of (quantity_name, [unit_names]) from the database.
pub fn build_unit_groups(db: &UnitDatabase) -> Vec<(String, Vec<String>)> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    for name in db.unit_names() {
        if let Some(unit) = db.lookup(name) {
            if !seen.insert(unit.name.clone()) {
                continue;
            }
            let qty = quantity_name(&unit.dimensions)
                .unwrap_or("Other")
                .to_string();
            groups.entry(qty).or_default().insert(unit.name.clone());
        }
    }
    groups
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect()))
        .collect()
}

/// Handle `const <name>` — echo a physical constant as a quantity.
fn handle_const_command(name: &str, opts: &FormatOptions) {
    let const_db = constants::global();
    match const_db.lookup(name) {
        Some(c) => println!("{}", format::format_constant_info(c, opts)),
        None => {
            let t = Theme::new(opts.color);
            eprintln!("{}", t.err(&format!("Error: unknown constant: '{name}'")));
        }
    }
}

fn print_error(e: &crate::error::RUnitsError, opts: &FormatOptions) {
    let t = Theme::new(opts.color);
    eprintln!("{}", t.err(&format!("Error: {e}")));
}

/// Returns `~/.config/runits/` using XDG_CONFIG_HOME or HOME fallback.
fn config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        Some(PathBuf::from(xdg).join("runits"))
    } else {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".config").join("runits"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_arrow_delimiter() {
        let (s, t) = parse_repl_line("100 km/h -> m/s").unwrap();
        assert_eq!(s, "100 km/h");
        assert_eq!(t, "m/s");
    }

    #[test]
    fn parse_to_delimiter() {
        let (s, t) = parse_repl_line("98.6 degF to degC").unwrap();
        assert_eq!(s, "98.6 degF");
        assert_eq!(t, "degC");
    }

    #[test]
    fn parse_in_delimiter() {
        let (s, t) = parse_repl_line("10 ft in m").unwrap();
        assert_eq!(s, "10 ft");
        assert_eq!(t, "m");
    }

    #[test]
    fn parse_empty_source_returns_none() {
        assert!(parse_repl_line(" -> m/s").is_none());
    }

    #[test]
    fn parse_no_delimiter_returns_none() {
        assert!(parse_repl_line("100 km/h m/s").is_none());
    }

    #[test]
    fn parse_trims_whitespace() {
        let (s, t) = parse_repl_line("  100 km/h  ->  m/s  ").unwrap();
        assert_eq!(s, "100 km/h");
        assert_eq!(t, "m/s");
    }

    // ---- ? help detection tests ----

    #[test]
    fn strip_question_prefix() {
        assert_eq!(strip_question_mark("? meter"), Some("meter"));
    }

    #[test]
    fn strip_question_suffix() {
        assert_eq!(strip_question_mark("meter ?"), Some("meter"));
    }

    #[test]
    fn strip_question_bare_returns_none() {
        assert!(strip_question_mark("?").is_none());
    }

    #[test]
    fn question_target_in_delimiter() {
        let (s, t) = parse_repl_line("100 km/h -> ?").unwrap();
        assert_eq!(s, "100 km/h");
        assert_eq!(t, "?");
    }

    // ---- match_command tests ----

    #[test]
    fn match_command_bare_matches_with_empty_args() {
        assert_eq!(match_command("explain", "explain"), Some(""));
        assert_eq!(match_command("info", "info"), Some(""));
    }

    #[test]
    fn match_command_with_args() {
        assert_eq!(match_command("const c_0", "const"), Some("c_0"));
        assert_eq!(match_command("list units", "list"), Some("units"));
    }

    #[test]
    fn match_command_trims_arg_whitespace() {
        assert_eq!(match_command("const   c_0  ", "const"), Some("c_0"));
    }

    #[test]
    fn match_command_rejects_prefix_without_boundary() {
        // "explainer" is not "explain" followed by whitespace
        assert_eq!(match_command("explainer", "explain"), None);
        // "listing" is not "list" followed by whitespace
        assert_eq!(match_command("listing", "list"), None);
    }

    #[test]
    fn match_command_rejects_unrelated_line() {
        assert_eq!(match_command("10 ft", "explain"), None);
    }

    // ---- resolve_list_subcommand tests ----

    #[test]
    fn resolve_list_subcommand_exact_matches() {
        assert_eq!(resolve_list_subcommand("units"), Some("units"));
        assert_eq!(resolve_list_subcommand("dimensions"), Some("dimensions"));
        assert_eq!(resolve_list_subcommand("constants"), Some("constants"));
    }

    #[test]
    fn resolve_list_subcommand_quantities_alias() {
        assert_eq!(resolve_list_subcommand("quantities"), Some("dimensions"));
    }

    #[test]
    fn resolve_list_subcommand_unique_single_letter_prefix() {
        assert_eq!(resolve_list_subcommand("u"), Some("units"));
        assert_eq!(resolve_list_subcommand("d"), Some("dimensions"));
        assert_eq!(resolve_list_subcommand("c"), Some("constants"));
        assert_eq!(resolve_list_subcommand("q"), Some("dimensions"));
    }

    #[test]
    fn resolve_list_subcommand_case_insensitive() {
        assert_eq!(resolve_list_subcommand("Units"), Some("units"));
        assert_eq!(resolve_list_subcommand("DIM"), Some("dimensions"));
    }

    #[test]
    fn resolve_list_subcommand_unknown_returns_none() {
        assert_eq!(resolve_list_subcommand("foo"), None);
        assert_eq!(resolve_list_subcommand(""), None);
    }
}
