//! Rustyline helper: tab-completion, inline hints, syntax highlighting.

use crate::database::UnitDatabase;
use crate::parser;
use crate::theme::Theme;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow;

/// Rustyline helper providing tab-completion of unit names.
pub(super) struct UnitsHelper {
    pub(super) db: &'static UnitDatabase,
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

        let t = Theme::new(true);
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
        let t = Theme::new(true);
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
