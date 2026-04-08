//! Output formatting: composes precision, color, Unicode, annotations, and JSON
//! into a single presentation layer.
//!
//! All conversion display goes through [`format_result`]. The caller builds
//! [`FormatOptions`] from CLI flags, config, and environment (isatty, `NO_COLOR`).
//!
//! Colors use semantic roles via [`Theme`] — a unit name is always "unit color"
//! whether it appears in a conversion result, `?` help, or REPL highlighting.
//! FUTURE: load theme from config.toml `[theme]` section with hex support.

use crate::convert::ConversionResult;
use crate::database::SI_PREFIXES;
use crate::units::quantity::{format_value, format_value_inner};
use owo_colors::Style;

// ---------------------------------------------------------------------------
// Theme — semantic color roles
// ---------------------------------------------------------------------------

/// Semantic color roles for consistent styling across all output.
///
/// Defaults are Flexoki-inspired ANSI colors. FUTURE: loadable from
/// config.toml `[theme]` with hex/truecolor support.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Unit names and symbols: meter, ft, N, kg·m·s⁻²
    pub unit: Style,
    /// Dimension analysis symbols: L, M, T, Θ
    pub dimension: Style,
    /// Numeric values: 0.3048, 1000, 10³
    pub number: Style,
    /// Conversion keywords: ->, to, in, as
    pub keyword: Style,
    /// Important label values: Force, Velocity, Reference unit
    pub label_value: Style,
    /// Secondary info: aliases, compatible list, system tag, hints, + SI prefixes
    pub dimmed: Style,
    /// Error messages
    pub error: Style,
}

impl Default for Theme {
    /// Flexoki-inspired ANSI defaults.
    fn default() -> Self {
        Self {
            unit: Style::new().cyan(),
            dimension: Style::new().blue().bold(),
            number: Style::new().yellow(),
            keyword: Style::new().bold(),
            label_value: Style::new().bold(),
            dimmed: Style::new().dimmed(),
            error: Style::new().red(),
        }
    }
}

impl Theme {
    /// Apply a style to text, respecting color enable flag.
    pub fn paint(&self, text: &str, style: &Style, color: bool) -> String {
        if color {
            format!("{}", style.style(text))
        } else {
            text.to_string()
        }
    }

    // Convenience methods for common roles.
    pub fn unit(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.unit, color)
    }
    pub fn dim_sym(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.dimension, color)
    }
    pub fn num(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.number, color)
    }
    pub fn kw(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.keyword, color)
    }
    pub fn lbl(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.label_value, color)
    }
    pub fn dim(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.dimmed, color)
    }
    pub fn err(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.error, color)
    }
}

/// Global default theme. FUTURE: replace with config-loaded theme.
pub fn default_theme() -> Theme {
    Theme::default()
}

// ---------------------------------------------------------------------------
// FormatOptions
// ---------------------------------------------------------------------------

/// Presentation settings resolved from CLI flags + config + environment.
#[derive(Debug, Clone, Default)]
pub struct FormatOptions {
    pub precision: Option<usize>,
    pub scientific: bool,
    pub to_base: bool,
    pub color: bool,
    pub unicode: bool,
    pub annotations: bool,
    pub json: bool,
}

impl FormatOptions {
    pub fn repl_defaults() -> Self {
        Self {
            color: true,
            unicode: true,
            annotations: true,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Conversion result formatting
// ---------------------------------------------------------------------------

/// Format a conversion result according to the given options.
pub fn format_result(result: &ConversionResult, opts: &FormatOptions) -> String {
    if opts.json {
        return format_json(result, opts);
    }

    let t = default_theme();
    let c = opts.color;

    let sig_figs = opts.precision.unwrap_or(6);
    let exact = opts.precision.is_some();

    let value_str = if exact {
        format_value_inner(result.result.value, sig_figs, opts.scientific, true)
    } else {
        format_value(result.result.value, sig_figs, opts.scientific)
    };

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

    let mut out = format!("{} {}", t.num(&value_str, c), t.unit(&unit_name, c));

    if opts.annotations
        && let Some(ann) = result.annotation
    {
        out.push_str(&format!(" {}", t.dim(&format!("[{}]", ann), c)));
    }

    out
}

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

// ---------------------------------------------------------------------------
// Unit info formatting (? help)
// ---------------------------------------------------------------------------

/// Format unit info for the `?` help query in the REPL.
pub fn format_unit_info(
    unit: &crate::units::Unit,
    aliases: &[String],
    compatible: &[String],
    annotation: Option<&str>,
    opts: &FormatOptions,
) -> String {
    let t = default_theme();
    let c = opts.color;
    let mut lines = Vec::new();

    let uni = |s: &str| -> String {
        if opts.unicode {
            unicode_unit_name(s)
        } else {
            s.to_string()
        }
    };

    // Header: name (aliases)
    let alias_str = if aliases.is_empty() {
        String::new()
    } else {
        format!(" {}", t.dim(&format!("({})", aliases.join(", ")), c))
    };
    lines.push(format!("{}{}", t.unit(&unit.name, c), alias_str));

    // Quantity (from annotation registry)
    if let Some(ann) = annotation {
        lines.push(format!("  Quantity: {}", t.lbl(ann, c)));
    }

    // Dimensions: abstract analysis formula
    let analysis = uni(&unit.analysis_string());
    lines.push(format!("  Dimensions: {}", t.dim_sym(&analysis, c)));

    // FUTURE(unit-systems): "[SI]" is hardcoded.
    let base_fmt = uni(&unit.to_base_unit_string());
    lines.push(format!(
        "  Base unit: {}  {}",
        t.unit(&base_fmt, c),
        t.dim("[SI]", c)
    ));

    // Factor / status / affine
    match &unit.conversion {
        crate::units::unit::ConversionKind::Linear(f) if (*f - 1.0).abs() < 1e-15 => {
            lines.push(format!("  {}", t.lbl("Reference unit", c)));
        }
        crate::units::unit::ConversionKind::Linear(f) => {
            let val = format_value(*f, 6, false);
            lines.push(format!(
                "  Factor: {} {} = {} {}",
                t.num("1", c),
                t.unit(&unit.name, c),
                t.num(&val, c),
                t.unit(&base_fmt, c),
            ));
        }
        crate::units::unit::ConversionKind::Affine { scale, offset } => {
            lines.push(format!(
                "  Affine: {} = value × {} + {}",
                t.unit("K", c),
                t.num(&scale.to_string(), c),
                t.num(&offset.to_string(), c),
            ));
        }
    }

    // Prefix info
    if let Some((prefix_name, scale)) = detect_si_prefix(&unit.name) {
        let exp = scale.log10().round() as i32;
        let exp_fmt = uni(&format!("^{}", exp));
        lines.push(format!(
            "  Prefix: {} ({}{})",
            t.unit(prefix_name, c),
            t.num("10", c),
            t.num(&exp_fmt, c),
        ));
    }

    // Compatible units — each name is a unit, so styled consistently
    if !compatible.is_empty() {
        let list = compatible
            .iter()
            .map(|u| t.unit(u, c))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  Compatible: {}", list));
    }

    // SI prefix note
    if unit.prefixable {
        lines.push(format!("  {}", t.dim("+ SI prefixes", c)));
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Unicode rendering
// ---------------------------------------------------------------------------

/// Transform ASCII compound-unit names to Unicode.
///
/// `*` → `·` (middle dot), `^N` → superscript digits.
/// No-op on simple names like "meter".
pub fn unicode_unit_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => out.push('\u{00B7}'),
            '^' => {
                while let Some(&next) = chars.peek() {
                    if next == '-' {
                        out.push('\u{207B}');
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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Detect if a unit name starts with a known SI prefix.
fn detect_si_prefix(name: &str) -> Option<(&'static str, f64)> {
    // FUTURE(alias-types): when units carry metadata about their prefix,
    // this detection becomes unnecessary.
    for &(long, _short, scale) in SI_PREFIXES {
        if let Some(remainder) = name.strip_prefix(long)
            && !remainder.is_empty()
        {
            return Some((long, scale));
        }
    }
    None
}

/// Determine whether to use color based on environment.
pub fn should_color(is_tty: bool) -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    is_tty
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

    #[test]
    fn theme_paint_no_color() {
        let t = Theme::default();
        assert_eq!(t.unit("meter", false), "meter");
        assert_eq!(t.num("3.14", false), "3.14");
    }
}
