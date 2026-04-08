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
use crate::units::dimension::Dimension;
use crate::units::quantity::{format_value, format_value_inner};
use owo_colors::Style;

// ---------------------------------------------------------------------------
// Theme — semantic color roles
// ---------------------------------------------------------------------------

/// Dimension-based color theme for consistent styling.
///
/// Each base dimension has its own color. Units inherit color from their
/// dimension (single-dimension) or use the compound style (multi-dimension).
/// Flexoki-inspired ANSI defaults. FUTURE: loadable from config.toml.
#[derive(Debug, Clone)]
pub struct Theme {
    // Per-dimension colors
    pub length: Style,
    pub mass: Style,
    pub time: Style,
    pub temperature: Style,
    pub current: Style,
    pub amount: Style,
    pub intensity: Style,
    pub angle: Style,
    pub information: Style,
    pub currency: Style,
    // Compound/derived quantity color (Force, Velocity, etc.)
    pub compound: Style,
    // Utility styles
    pub number: Style,
    pub keyword: Style,
    pub dimmed: Style,
    pub error: Style,
}

impl Default for Theme {
    /// Flexoki-inspired ANSI defaults.
    fn default() -> Self {
        Self {
            length: Style::new().blue(),
            mass: Style::new().red(),
            time: Style::new().green(),
            temperature: Style::new().truecolor(218, 112, 44), // Flexoki orange
            current: Style::new().yellow(),
            amount: Style::new().magenta(),
            intensity: Style::new().bright_magenta(),
            angle: Style::new().cyan(),
            information: Style::new().bright_blue(),
            currency: Style::new().bright_yellow(),
            compound: Style::new().bright_white().bold(),
            number: Style::new().yellow(),
            keyword: Style::new().dimmed().bold(),
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

    /// Style for a specific dimension.
    pub fn dimension_style(&self, dim: &Dimension) -> &Style {
        match dim {
            Dimension::Length => &self.length,
            Dimension::Mass => &self.mass,
            Dimension::Time => &self.time,
            Dimension::Temperature => &self.temperature,
            Dimension::Current => &self.current,
            Dimension::AmountOfSubstance => &self.amount,
            Dimension::LuminousIntensity => &self.intensity,
            Dimension::Angle => &self.angle,
            Dimension::Information => &self.information,
            Dimension::Currency => &self.currency,
        }
    }

    /// Style for a unit based on its dimensions.
    /// Single-dimension → that dimension's color.
    /// Multi-dimension (compound) → compound style.
    /// Dimensionless → compound style.
    pub fn unit_style(&self, unit: &crate::units::Unit) -> &Style {
        if unit.dimensions.len() == 1 {
            let (dim, _) = unit.dimensions.iter().next().unwrap();
            self.dimension_style(dim)
        } else {
            &self.compound
        }
    }

    // Convenience methods.
    pub fn unit_text(&self, text: &str, unit: &crate::units::Unit, color: bool) -> String {
        self.paint(text, self.unit_style(unit), color)
    }
    pub fn num(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.number, color)
    }
    pub fn kw(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.keyword, color)
    }
    pub fn lbl(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.compound, color) // labels use compound/bold style
    }
    pub fn dim(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.dimmed, color)
    }
    pub fn err(&self, text: &str, color: bool) -> String {
        self.paint(text, &self.error, color)
    }
}

/// Render a dimension map with per-dimension coloring.
///
/// Each symbol is colored by its dimension; exponents use number color.
/// Separators use `·` (unicode) or `*` (ascii).
pub fn colored_dimensions(
    dims: &crate::units::dimension::DimensionMap,
    symbol_fn: fn(&Dimension) -> &str,
    unicode: bool,
    theme: &Theme,
    color: bool,
) -> String {
    use crate::units::unit::Unit;
    // Sort: positive exponents first, then alphabetical by symbol.
    let mut entries: Vec<_> = dims.iter().map(|(d, &e)| (d, symbol_fn(d), e)).collect();
    entries.sort_by(|a, b| b.2.signum().cmp(&a.2.signum()).then(a.1.cmp(b.1)));

    let sep = if unicode { "\u{00B7}" } else { "*" };
    let parts: Vec<String> = entries
        .iter()
        .map(|(dim, sym, exp)| {
            let styled_sym = theme.paint(sym, theme.dimension_style(dim), color);
            if *exp == 1 {
                styled_sym
            } else {
                let exp_str = if unicode {
                    unicode_unit_name(&format!("^{}", exp))
                } else {
                    format!("^{}", exp)
                };
                format!("{}{}", styled_sym, theme.num(&exp_str, color))
            }
        })
        .collect();

    if parts.is_empty() {
        // Use a dummy dimensionless unit to pick style
        let _ = Unit::dimensionless();
        "dimensionless".to_string()
    } else {
        parts.join(sep)
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

    let mut out = format!(
        "{} {}",
        t.num(&value_str, c),
        t.unit_text(&unit_name, &result.result.unit, c)
    );

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

    // Header: name (aliases) — colored by unit's dimension
    let alias_str = if aliases.is_empty() {
        String::new()
    } else {
        format!(" {}", t.dim(&format!("({})", aliases.join(", ")), c))
    };
    lines.push(format!("{}{}", t.unit_text(&unit.name, unit, c), alias_str));

    // Quantity (from annotation registry) — same color as the unit
    if let Some(ann) = annotation {
        lines.push(format!(
            "  {} {}",
            t.dim("Quantity:", c),
            t.unit_text(ann, unit, c)
        ));
    }

    // Dimensions: each symbol colored by its dimension
    let dims_colored = colored_dimensions(
        &unit.dimensions,
        Dimension::analysis_symbol,
        opts.unicode,
        &t,
        c,
    );
    lines.push(format!("  {} {}", t.dim("Dimensions:", c), dims_colored));

    // FUTURE(unit-systems): "[SI]" is hardcoded.
    // Base unit: each component colored by its dimension
    let base_colored = colored_dimensions(
        &unit.dimensions,
        Dimension::base_symbol,
        opts.unicode,
        &t,
        c,
    );
    lines.push(format!(
        "  {} {}  {}",
        t.dim("Base unit:", c),
        base_colored,
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
                "  {} {} {} = {} {}",
                t.dim("Factor:", c),
                t.num("1", c),
                t.unit_text(&unit.name, unit, c),
                t.num(&val, c),
                base_colored,
            ));
        }
        crate::units::unit::ConversionKind::Affine { scale, offset } => {
            lines.push(format!(
                "  {} {} = value × {} + {}",
                t.dim("Affine:", c),
                t.paint("K", t.dimension_style(&Dimension::Temperature), c),
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
            "  {} {} ({}{})",
            t.dim("Prefix:", c),
            t.unit_text(prefix_name, unit, c),
            t.num("10", c),
            t.num(&exp_fmt, c),
        ));
    }

    // Compatible units — same dimensions as queried unit, same color
    if !compatible.is_empty() {
        let style = t.unit_style(unit);
        let list = compatible
            .iter()
            .map(|u| t.paint(u, style, c))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  {} {}", t.dim("Compatible:", c), list));
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
        let meter = crate::units::Unit::meter();
        assert_eq!(t.unit_text("meter", &meter, false), "meter");
        assert_eq!(t.num("3.14", false), "3.14");
    }
}
