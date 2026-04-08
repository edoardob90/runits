//! Interactive REPL mode.
//!
//! Entered when `runits` is called with no arguments. Each line is a
//! conversion query in the form `<quantity> -> <target>` (also accepts
//! `to`, `in`, `as` delimiters). History is persisted at
//! `~/.config/runits/history`.

use crate::annotations::quantity_name;
use crate::database::{self, UnitDatabase};
use crate::format::{self, FormatOptions};
use crate::parser;
use crate::{Dimension, convert};
use rustyline::Editor;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow;
use std::path::PathBuf;

/// Rustyline helper providing tab-completion of unit names.
struct UnitsHelper {
    db: &'static UnitDatabase,
}

impl Completer for UnitsHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Find the start of the current word: scan back from cursor to last delimiter.
        let word_start = line[..pos]
            .rfind([' ', '*', '/', '(', '^', '?'])
            .map(|i| i + 1)
            .unwrap_or(0);

        let partial = &line[word_start..pos];

        // Dimension-aware: if we're past a delimiter (-> / to / in / as),
        // try to parse the source side and filter to compatible units only.
        let compatible_filter = self.source_dimensions(line);

        // Empty partial: show all compatible units if we have a dimension filter
        // (user typed "10 m to " and wants to see options). Otherwise skip.
        if (partial.is_empty() || partial == "?") && compatible_filter.is_none() {
            return Ok((pos, vec![]));
        }

        // Skip if partial looks like a number (don't complete digits).
        if partial.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.') {
            return Ok((pos, vec![]));
        }

        let mut matches: Vec<Pair> = self
            .db
            .unit_names()
            .filter(|name| name.starts_with(partial))
            .filter(|name| {
                // If we know the source dimensions, only suggest compatible units.
                match &compatible_filter {
                    Some(dims) => self.db.lookup(name).is_some_and(|u| u.dimensions == *dims),
                    None => true, // No source context — suggest all.
                }
            })
            .map(|name| Pair {
                display: name.to_string(),
                replacement: name.to_string(),
            })
            .collect();
        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches.truncate(20);

        Ok((word_start, matches))
    }
}

impl UnitsHelper {
    /// If the cursor is past a conversion delimiter, try to parse the source
    /// side and return its dimensions for filtering completions.
    fn source_dimensions(&self, line: &str) -> Option<crate::units::dimension::DimensionMap> {
        // Try each delimiter; use the first one found.
        for delim in [" -> ", " to ", " in ", " as "] {
            if let Some(delim_pos) = line.find(delim) {
                let source = line[..delim_pos].trim();
                if source.is_empty() {
                    return None;
                }
                // Try parsing the source as a quantity to extract dimensions.
                if let Ok(qty) = parser::parse_quantity(source, self.db) {
                    return Some(qty.unit.dimensions.clone());
                }
                // Source didn't parse (incomplete/invalid) — no filter.
                return None;
            }
        }
        None // No delimiter found — source side, no filter.
    }
}

impl Hinter for UnitsHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Option<String> {
        // Only hint at end of line (not mid-edit).
        if pos < line.len() {
            return None;
        }

        let word_start = line[..pos]
            .rfind([' ', '*', '/', '(', '^', '?'])
            .map(|i| i + 1)
            .unwrap_or(0);
        let partial = &line[word_start..pos];
        if partial.is_empty()
            || partial.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.')
        {
            return None;
        }

        let dims = self.source_dimensions(line);

        // Find shortest prefix match — most likely intended unit.
        self.db
            .unit_names()
            .filter(|name| name.starts_with(partial))
            .filter(|name| match &dims {
                Some(d) => self.db.lookup(name).is_some_and(|u| u.dimensions == *d),
                None => true,
            })
            .min_by_key(|name| name.len())
            .map(|name| name[partial.len()..].to_string())
    }
}

impl Highlighter for UnitsHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if std::env::var_os("NO_COLOR").is_some() {
            return Cow::Borrowed(line);
        }

        let t = format::Theme::new(true);
        let mut result = String::with_capacity(line.len() + 64);
        let mut i = 0;
        let bytes = line.as_bytes();

        while i < bytes.len() {
            if bytes[i] == b' ' || bytes[i] == b'\t' {
                result.push(bytes[i] as char);
                i += 1;
                continue;
            }

            // "->" delimiter
            if i + 1 < bytes.len() && &line[i..i + 2] == "->" {
                result.push_str(&t.kw("->"));
                i += 2;
                continue;
            }

            // Tokenize until next whitespace.
            let start = i;
            while i < bytes.len() && bytes[i] != b' ' && bytes[i] != b'\t' {
                i += 1;
            }
            let token = &line[start..i];

            if token == "to" || token == "in" || token == "as" {
                result.push_str(&t.kw(token));
            } else if token.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.') {
                result.push_str(&t.num(token));
            } else if token == "?" || token == "quit" || token == "exit" {
                result.push_str(token);
            } else if let Some(u) = self.db.lookup(token) {
                result.push_str(&t.unit_text(token, &u));
            } else {
                result.push_str(token);
            }
        }

        Cow::Owned(result)
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        let t = format::Theme::new(true);
        Cow::Owned(t.dim(hint))
    }

    fn highlight_char(
        &self,
        _line: &str,
        _pos: usize,
        _kind: rustyline::highlight::CmdKind,
    ) -> bool {
        true
    }
}
impl Validator for UnitsHelper {}
impl Helper for UnitsHelper {}

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

    let t = format::Theme::new(opts.color);
    let db = database::global();

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
                if line == "info" {
                    print_info(&t, db);
                    continue;
                }
                let _ = rl.add_history_entry(line);

                handle_input(line, db, opts);
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
fn print_banner(mode: crate::cli::BannerMode, t: &format::Theme, db: &UnitDatabase) {
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

fn print_info(t: &format::Theme, db: &UnitDatabase) {
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

fn handle_input(line: &str, db: &UnitDatabase, opts: &FormatOptions) {
    // 1. Delimiter-based input (check first, so "100 km/h -> ?" is caught).
    if let Some((source, target)) = parse_repl_line(line) {
        if target == "?" {
            handle_quantity_help(source, db, opts);
        } else {
            match convert::run_conversion(source, target, db) {
                Ok(result) => println!("{}", format::format_result(&result, opts)),
                Err(e) => print_error(&e, opts),
            }
        }
        return;
    }

    // 2. ? prefix/suffix: "? meter", "meter ?", "1 N ?"
    //    If query contains a digit → it's a quantity ("1 N ?", "2.5 ft ?").
    //    Otherwise → unit help ("? meter", "N ?").
    if let Some(query) = strip_question_mark(line) {
        let has_number = query.contains(|c: char| c.is_ascii_digit());
        if has_number && let Ok(qty) = parser::parse_quantity(query, db) {
            handle_quantity_help_from_qty(qty, db, opts);
            return;
        }
        handle_unit_help(query, db, opts);
        return;
    }

    // 3. No delimiter, no ? — try parsing as a bare quantity.
    match parser::parse_quantity(line, db) {
        Ok(qty) => {
            let annotation = quantity_name(&qty.unit.dimensions);
            let result = convert::ConversionResult {
                result: qty,
                annotation,
            };
            println!("{}", format::format_result(&result, opts));
        }
        Err(e) => print_error(&e, opts),
    }
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
        Err(e) => print_error(&e, opts),
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
        result: qty.clone(),
        annotation,
    };
    println!("{}", format::format_result(&result, opts));

    let t = format::Theme::new(opts.color);
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

fn print_error(e: &crate::error::RUnitsError, opts: &FormatOptions) {
    let t = format::Theme::new(opts.color);
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
}
