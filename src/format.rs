//! Output formatting: composes precision, color, Unicode, annotations, and JSON
//! into a single presentation layer.
//!
//! All conversion display goes through [`format_result`]. The caller builds
//! [`FormatOptions`] from CLI flags, config, and environment (isatty, `NO_COLOR`).

use crate::convert::ConversionResult;
use crate::units::quantity::{format_value, format_value_inner};
use owo_colors::OwoColorize;

/// Presentation settings resolved from CLI flags + config + environment.
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// Significant figures (None = default 6).
    pub precision: Option<usize>,
    /// Force scientific notation.
    pub scientific: bool,
    /// Expand unit to base SI symbols.
    pub to_base: bool,
    /// Use ANSI colors in output.
    pub color: bool,
    /// Use Unicode symbols (middle-dot, superscript exponents).
    pub unicode: bool,
    /// Show physical-quantity annotations (e.g. "Velocity", "Force").
    pub annotations: bool,
    /// Output as JSON.
    pub json: bool,
}

impl Default for FormatOptions {
    /// Defaults for one-shot CLI: no color, no unicode, no annotations.
    fn default() -> Self {
        Self {
            precision: None,
            scientific: false,
            to_base: false,
            color: false,
            unicode: false,
            annotations: false,
            json: false,
        }
    }
}

impl FormatOptions {
    /// Defaults for REPL mode: color + unicode + annotations on.
    pub fn repl_defaults() -> Self {
        Self {
            color: true,
            unicode: true,
            annotations: true,
            ..Default::default()
        }
    }
}

/// Format a conversion result according to the given options.
pub fn format_result(result: &ConversionResult, opts: &FormatOptions) -> String {
    if opts.json {
        return format_json(result, opts);
    }

    let sig_figs = opts.precision.unwrap_or(6);
    let exact = opts.precision.is_some();

    // Format value.
    let value_str = if exact {
        format_value_inner(result.result.value, sig_figs, opts.scientific, true)
    } else {
        format_value(result.result.value, sig_figs, opts.scientific)
    };

    // Format unit name.
    let raw_name = if opts.to_base {
        result.result.unit.to_base_unit_string()
    } else {
        result.result.unit.name.clone()
    };
    let unit_name = if opts.unicode {
        unicode_unit_name(&raw_name)
    } else {
        raw_name
    };

    // Build the output string, with optional color + annotation.
    if opts.color {
        let mut out = format!("{} {}", value_str.bold(), unit_name.cyan());
        if opts.annotations
            && let Some(ann) = result.annotation
        {
            out.push_str(&format!(" {}", format!("[{}]", ann).dimmed()));
        }
        out
    } else {
        let mut out = format!("{} {}", value_str, unit_name);
        if opts.annotations
            && let Some(ann) = result.annotation
        {
            out.push_str(&format!(" [{}]", ann));
        }
        out
    }
}

/// Format as JSON (no color, no unicode).
fn format_json(result: &ConversionResult, opts: &FormatOptions) -> String {
    let sig_figs = opts.precision.unwrap_or(6);
    let value_str = format_value(result.result.value, sig_figs, false);
    let annotation = result
        .annotation
        .map(|a| format!("\"{}\"", a))
        .unwrap_or_else(|| "null".to_string());
    format!(
        "{{\"value\":{},\"unit\":\"{}\",\"annotation\":{}}}",
        value_str, result.result.unit.name, annotation,
    )
}

/// Transform ASCII compound-unit names to Unicode.
///
/// `*` → `·` (middle dot), `^N` → superscript digits.
/// No-op on simple names like "meter".
pub fn unicode_unit_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => out.push('\u{00B7}'), // middle dot
            '^' => {
                // Consume the exponent: optional minus + digits.
                while let Some(&next) = chars.peek() {
                    if next == '-' {
                        out.push('\u{207B}'); // superscript minus
                        chars.next();
                    } else if next.is_ascii_digit() {
                        out.push(superscript_digit(next));
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn superscript_digit(d: char) -> char {
    match d {
        '0' => '\u{2070}',
        '1' => '\u{00B9}',
        '2' => '\u{00B2}',
        '3' => '\u{00B3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        _ => d,
    }
}

/// Format unit info for the `?` help query in the REPL.
///
/// Layout:
/// ```text
/// newton (N, newtons)
///   Quantity: Force
///   Dimensions: M·L·T⁻²  →  kg·m·s⁻²
///   Compatible: dyne, kilogram_force, pound_force
///   + SI prefixes
/// ```
pub fn format_unit_info(
    unit: &crate::units::Unit,
    aliases: &[String],
    compatible: &[String],
    annotation: Option<&str>,
    opts: &FormatOptions,
) -> String {
    let mut lines = Vec::new();

    // Header: name (aliases)
    let alias_str = if aliases.is_empty() {
        String::new()
    } else {
        format!(" ({})", aliases.join(", "))
    };
    if opts.color {
        lines.push(format!("{}{}", unit.name.cyan().bold(), alias_str.dimmed()));
    } else {
        lines.push(format!("{}{}", unit.name, alias_str));
    }

    // Quantity (from annotation registry — dynamic)
    if let Some(ann) = annotation {
        if opts.color {
            lines.push(format!("  Quantity: {}", ann.bold()));
        } else {
            lines.push(format!("  Quantity: {}", ann));
        }
    }

    // FUTURE(unit-systems): "[SI]" is hardcoded. When CGS/natural units land,
    // derive the system label from the active unit system context.
    // Dimensions: analysis symbols → base unit symbols
    let analysis = unit.analysis_string();
    let base = unit.to_base_unit_string();
    let (analysis_fmt, base_fmt) = if opts.unicode {
        (unicode_unit_name(&analysis), unicode_unit_name(&base))
    } else {
        (analysis, base)
    };
    // System label hardcoded to SI; Phase 5 makes it dynamic.
    if opts.color {
        lines.push(format!(
            "  Dimensions: {}  →  {}  {}",
            analysis_fmt,
            base_fmt,
            "[SI]".dimmed()
        ));
    } else {
        lines.push(format!(
            "  Dimensions: {}  →  {}  [SI]",
            analysis_fmt, base_fmt
        ));
    }

    // Base/derived + affine info
    match &unit.conversion {
        crate::units::unit::ConversionKind::Linear(f) if (*f - 1.0).abs() < 1e-15 => {
            lines.push("  Reference unit".to_string());
        }
        crate::units::unit::ConversionKind::Affine { scale, offset } => {
            lines.push(format!("  Affine: K = value × {} + {}", scale, offset));
        }
        _ => {} // Derived linear — no extra line, user can convert to see factor.
    }

    // Compatible units
    if !compatible.is_empty() {
        if opts.color {
            lines.push(format!("  Compatible: {}", compatible.join(", ").dimmed()));
        } else {
            lines.push(format!("  Compatible: {}", compatible.join(", ")));
        }
    }

    // SI prefix note for linear units
    if !unit.is_affine() {
        if opts.color {
            lines.push(format!("  {}", "+ SI prefixes".dimmed()));
        } else {
            lines.push("  + SI prefixes".to_string());
        }
    }

    lines.join("\n")
}

/// Determine whether to use color based on environment.
pub fn should_color(is_tty: bool) -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    is_tty
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::run_conversion;
    use crate::database::UnitDatabase;

    #[test]
    fn format_plain_no_annotation() {
        let db = UnitDatabase::new();
        let r = run_conversion("10 ft", "m", &db).unwrap();
        let opts = FormatOptions::default();
        let out = format_result(&r, &opts);
        assert_eq!(out, "3.048 meter");
    }

    #[test]
    fn format_with_annotation() {
        let db = UnitDatabase::new();
        let r = run_conversion("100 km/h", "m/s", &db).unwrap();
        let opts = FormatOptions {
            annotations: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(out.contains("meter/second"));
        assert!(out.contains("[Velocity]"));
    }

    #[test]
    fn format_json_output() {
        let db = UnitDatabase::new();
        let r = run_conversion("10 ft", "m", &db).unwrap();
        let opts = FormatOptions {
            json: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(out.contains("\"value\":3.048"));
        assert!(out.contains("\"unit\":\"meter\""));
    }

    #[test]
    fn unicode_multiplication() {
        assert_eq!(unicode_unit_name("kg*m"), "kg\u{00B7}m");
    }

    #[test]
    fn unicode_exponent() {
        assert_eq!(unicode_unit_name("s^2"), "s\u{00B2}");
        assert_eq!(unicode_unit_name("m^-1"), "m\u{207B}\u{00B9}");
    }

    #[test]
    fn unicode_compound() {
        assert_eq!(unicode_unit_name("kg*m/s^2"), "kg\u{00B7}m/s\u{00B2}");
    }

    #[test]
    fn unicode_noop_on_simple_name() {
        assert_eq!(unicode_unit_name("meter"), "meter");
    }
}
