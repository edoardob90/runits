//! Output formatting: composes precision, color, Unicode, annotations, and JSON
//! into a single presentation layer.
//!
//! All conversion display goes through [`format_result`]. The caller builds
//! [`FormatOptions`] from CLI flags, config, and environment (isatty, `NO_COLOR`).
//!
//! Colors use semantic roles via [`Theme`] — a unit name
//! is always "unit color" whether it appears in a conversion result, `?` help,
//! or REPL highlighting.

use crate::convert::ConversionResult;
use crate::database::SI_PREFIXES;
use crate::theme::Theme;
use crate::units::dimension::Dimension;
use crate::units::quantity::{format_value, format_value_inner};

/// FUTURE(unit-systems): this becomes dynamic when CGS/natural units land.
pub const UNIT_SYSTEM: &str = "SI";

// ---------------------------------------------------------------------------
// Glyphs: ASCII vs Unicode presentational symbols
// ---------------------------------------------------------------------------

/// Presentational glyphs for arithmetic and unit rendering.
///
/// Two fixed instances live as associated constants so the right set is
/// picked by `opts.unicode` without allocation. Centralizes the ASCII↔Unicode
/// choice that would otherwise be scattered as `if unicode { "→" } else { "->" }`
/// throughout the formatting code.
pub(crate) struct Glyphs {
    /// Conversion arrow: `->` or `→`.
    pub arrow: &'static str,
    /// Multiplication: `*` or `×`.
    pub times: &'static str,
    /// Division: `/` or `÷`.
    pub divide: &'static str,
    /// Minus sign: `-` or `−` (true minus, not hyphen).
    pub minus: &'static str,
    /// Compound-unit separator: `*` or `·` (middle dot).
    pub dot: &'static str,
}

impl Glyphs {
    pub const ASCII: Self = Self {
        arrow: "->",
        times: "*",
        divide: "/",
        minus: "-",
        dot: "*",
    };
    pub const UNICODE: Self = Self {
        arrow: "\u{2192}",  // →
        times: "\u{00D7}",  // ×
        divide: "\u{00F7}", // ÷
        minus: "\u{2212}",  // −
        dot: "\u{00B7}",    // ·
    };

    /// Pick the glyph set for the current presentation mode.
    pub fn pick(unicode: bool) -> &'static Self {
        if unicode {
            &Self::UNICODE
        } else {
            &Self::ASCII
        }
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
) -> String {
    use crate::units::unit::Unit;
    // Sort: positive exponents first, then alphabetical by symbol.
    let mut entries: Vec<_> = dims.iter().map(|(d, &e)| (d, symbol_fn(d), e)).collect();
    entries.sort_by(|a, b| b.2.signum().cmp(&a.2.signum()).then(a.1.cmp(b.1)));

    let g = Glyphs::pick(unicode);
    let parts: Vec<String> = entries
        .iter()
        .map(|(dim, sym, exp)| {
            let styled_sym = theme.paint(sym, theme.dimension_style(dim));
            if *exp == 1 {
                styled_sym
            } else {
                let exp_str = if unicode {
                    unicode_unit_name(&format!("^{}", exp))
                } else {
                    format!("^{}", exp)
                };
                format!("{}{}", styled_sym, theme.num(&exp_str))
            }
        })
        .collect();

    if parts.is_empty() {
        // Use a dummy dimensionless unit to pick style
        let _ = Unit::dimensionless();
        "dimensionless".to_string()
    } else {
        parts.join(g.dot)
    }
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
    pub explain: bool,
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
    if opts.explain {
        return format_explain(result, opts);
    }

    let t = Theme::new(opts.color);

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
        t.num(&value_str),
        t.unit_text(&unit_name, &result.result.unit)
    );

    if opts.annotations
        && let Some(ann) = result.annotation
    {
        out.push_str(&format!(" {}", t.dim(&format!("[{}]", ann))));
    }

    out
}

/// Format a step-by-step conversion explanation.
///
/// Layout is uniform across linear and affine conversions:
///
/// ```text
/// <value> <src> → <tgt> [Annotation]
///   source:  <src_formula> = <base>
///   target:  <tgt_formula> = <base>
///
///       <source → base arithmetic>
///       <base → target arithmetic>
/// ```
///
/// `source:` / `target:` lines are skipped when that side is the base unit
/// (the formula would be trivial). The calculation steps use actual values
/// from the conversion, and are indented to stand out as the answer.
pub fn format_explain(result: &ConversionResult, opts: &FormatOptions) -> String {
    use crate::units::unit::ConversionKind;

    let t = Theme::new(opts.color);
    let g = Glyphs::pick(opts.unicode);
    let source = &result.source;
    let target = &result.result;
    let fv = |v: f64| format_value(v, 6, false);
    let uni = |s: &str| -> String {
        if opts.unicode {
            unicode_unit_name(s)
        } else {
            s.to_string()
        }
    };

    let base_str = uni(&source.unit.to_base_unit_string());
    let base_value = source.unit.to_base_value(source.value);

    // A side is "the base unit" when it's linear with factor 1. Skipping
    // its formula/calculation line avoids trivial `meter × 1 = meter` noise.
    let is_base = |u: &crate::units::Unit| matches!(&u.conversion, ConversionKind::Linear(f) if (*f - 1.0).abs() < 1e-15);
    let source_is_base = is_base(&source.unit);
    let target_is_base = is_base(&target.unit);

    // Formula line: describes how a unit converts to base.
    // Linear:  `<unit> × <factor> = <base>`
    // Affine:  `<unit> × <scale> + <offset> = <base>`
    // The unit name on the LHS is read as "value in this unit" — the common
    // physics-text shorthand.
    let formula = |unit: &crate::units::Unit| -> String {
        match &unit.conversion {
            ConversionKind::Linear(f) => format!(
                "{} {} {} {} {}",
                t.unit_text(&uni(&unit.name), unit),
                t.kw(g.times),
                t.num(&fv(*f)),
                t.kw("="),
                t.unit_text(&base_str, unit),
            ),
            ConversionKind::Affine { scale, offset } => format!(
                "{} {} {} + {} {} {}",
                t.unit_text(&uni(&unit.name), unit),
                t.kw(g.times),
                t.num(&fv(*scale)),
                t.num(&fv(*offset)),
                t.kw("="),
                t.unit_text(&base_str, unit),
            ),
        }
    };

    // Arithmetic step: source → base, with the actual source value plugged in.
    let to_base_step = |unit: &crate::units::Unit| -> String {
        match &unit.conversion {
            ConversionKind::Linear(f) => format!(
                "{} {} {} {} {} {}",
                t.num(&fv(source.value)),
                t.kw(g.times),
                t.num(&fv(*f)),
                t.kw("="),
                t.num(&fv(base_value)),
                t.unit_text(&base_str, unit),
            ),
            ConversionKind::Affine { scale, offset } => format!(
                "{} {} {} + {} {} {} {}",
                t.num(&fv(source.value)),
                t.kw(g.times),
                t.num(&fv(*scale)),
                t.num(&fv(*offset)),
                t.kw("="),
                t.num(&fv(base_value)),
                t.unit_text(&base_str, unit),
            ),
        }
    };

    // Arithmetic step: base → target, producing the final answer.
    let from_base_step = |unit: &crate::units::Unit| -> String {
        let target_name = uni(&unit.name);
        match &unit.conversion {
            ConversionKind::Linear(f) => format!(
                "{} {} {} {} {} {}",
                t.num(&fv(base_value)),
                t.kw(g.divide),
                t.num(&fv(*f)),
                t.kw("="),
                t.num(&fv(target.value)),
                t.unit_text(&target_name, unit),
            ),
            ConversionKind::Affine { scale, offset } => format!(
                "({} {} {}) {} {} {} {} {}",
                t.num(&fv(base_value)),
                t.kw(g.minus),
                t.num(&fv(*offset)),
                t.kw(g.divide),
                t.num(&fv(*scale)),
                t.kw("="),
                t.num(&fv(target.value)),
                t.unit_text(&target_name, unit),
            ),
        }
    };

    let mut lines = Vec::new();

    // Header: value source_unit → target_unit [Annotation]
    let ann_str = result
        .annotation
        .map(|a| format!(" {}", t.dim(&format!("[{}]", a))))
        .unwrap_or_default();
    lines.push(format!(
        "{} {} {} {}{}",
        t.num(&fv(source.value)),
        t.unit_text(&uni(&source.unit.name), &source.unit),
        t.kw(g.arrow),
        t.unit_text(&uni(&target.unit.name), &target.unit),
        ann_str,
    ));

    // Formula section: how each side relates to base (labels `source:` / `target:`).
    if !source_is_base {
        lines.push(format!("  {}  {}", t.dim("source:"), formula(&source.unit)));
    }
    if !target_is_base {
        lines.push(format!("  {}  {}", t.dim("target:"), formula(&target.unit)));
    }

    // Blank line separates the formula reference from the standout calculation.
    lines.push(String::new());

    // Calculation section: indented, values plugged in. One step per non-trivial
    // side. Identity conversions (both sides are the base unit) just echo the
    // result so the user sees *something* in the standout block.
    let indent = "    ";
    if !source_is_base {
        lines.push(format!("{}{}", indent, to_base_step(&source.unit)));
    }
    if !target_is_base {
        lines.push(format!("{}{}", indent, from_base_step(&target.unit)));
    }
    if source_is_base && target_is_base {
        lines.push(format!(
            "{}{} {} {}",
            indent,
            t.kw("="),
            t.num(&fv(target.value)),
            t.unit_text(&uni(&target.unit.name), &target.unit),
        ));
    }

    lines.join("\n")
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
    let t = Theme::new(opts.color);
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
        format!(" {}", t.dim(&format!("({})", aliases.join(", "))))
    };
    lines.push(format!("{}{}", t.unit_text(&unit.name, unit), alias_str));

    // Quantity (from annotation registry) — same color as the unit
    if let Some(ann) = annotation {
        lines.push(format!(
            "  {} {}",
            t.dim("Quantity:"),
            t.unit_text(ann, unit)
        ));
    }

    // Dimensions: each symbol colored by its dimension
    let dims_colored = colored_dimensions(
        &unit.dimensions,
        Dimension::analysis_symbol,
        opts.unicode,
        &t,
    );
    lines.push(format!("  {} {}", t.dim("Dimensions:"), dims_colored));

    // System base unit — FUTURE(unit-systems): UNIT_SYSTEM becomes dynamic
    let base_colored =
        colored_dimensions(&unit.dimensions, Dimension::base_symbol, opts.unicode, &t);
    let sys_label = format!("{} base:", UNIT_SYSTEM);
    lines.push(format!("  {} {}", t.dim(&sys_label), base_colored));

    // Factor / status / affine
    match &unit.conversion {
        crate::units::unit::ConversionKind::Linear(f) if (*f - 1.0).abs() < 1e-15 => {
            lines.push(format!("  {} {} (reference)", t.dim("Factor:"), t.num("1")));
        }
        crate::units::unit::ConversionKind::Linear(f) => {
            let val = format_value(*f, 6, false);
            lines.push(format!(
                "  {} {} {} = {} {}",
                t.dim("Factor:"),
                t.num("1"),
                t.unit_text(&unit.name, unit),
                t.num(&val),
                base_colored,
            ));
        }
        crate::units::unit::ConversionKind::Affine { scale, offset } => {
            lines.push(format!(
                "  {} {} = value × {} + {}",
                t.dim("Affine:"),
                t.paint("K", t.dimension_style(&Dimension::Temperature)),
                t.num(&scale.to_string()),
                t.num(&offset.to_string()),
            ));
        }
    }

    // Prefix info
    if let Some((prefix_name, scale)) = detect_si_prefix(&unit.name) {
        let exp = scale.log10().round() as i32;
        let exp_fmt = uni(&format!("^{}", exp));
        lines.push(format!(
            "  {} {} ({}{})",
            t.dim("Prefix:"),
            t.unit_text(prefix_name, unit),
            t.num("10"),
            t.num(&exp_fmt),
        ));
    }

    // Compatible units — same dimensions as queried unit, same color
    if !compatible.is_empty() {
        let style = t.unit_style(unit);
        let list = compatible
            .iter()
            .map(|u| t.paint(u, style))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  {} {}", t.dim("Compatible:"), list));
    }

    // SI prefix note
    if unit.prefixable {
        lines.push(format!("  {}", t.dim("+ SI prefixes")));
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Constant info formatting (? help for constants)
// ---------------------------------------------------------------------------

/// Format constant info for the `?` help query in the REPL.
pub fn format_constant_info(
    constant: &crate::database::constants::Constant,
    opts: &FormatOptions,
) -> String {
    let t = Theme::new(opts.color);
    let mut lines = Vec::new();

    // Header: name — styled as a constant (italic purple)
    lines.push(t.cst(constant.name));

    // Description
    lines.push(format!(
        "  {} {}",
        t.dim("Description:"),
        constant.description
    ));

    // Value + unit
    let val = format_value(constant.value, 10, false);
    let unit_name = if opts.unicode {
        unicode_unit_name(&constant.unit.name)
    } else {
        constant.unit.name.clone()
    };
    lines.push(format!(
        "  {} {} {}",
        t.dim("Value:"),
        t.num(&val),
        t.unit_text(&unit_name, &constant.unit),
    ));

    // Dimensions
    let dims_colored = colored_dimensions(
        &constant.unit.dimensions,
        Dimension::analysis_symbol,
        opts.unicode,
        &t,
    );
    if constant.unit.dimensions.is_empty() {
        lines.push(format!("  {} (dimensionless)", t.dim("Dimensions:")));
    } else {
        lines.push(format!("  {} {}", t.dim("Dimensions:"), dims_colored));
    }

    lines.join("\n")
}

// ---------------------------------------------------------------------------
// Unit list formatting (search / list-units)
// ---------------------------------------------------------------------------

/// Format a list of unit names for a given quantity, with dimension colors.
///
/// `dims` colors both the quantity header and unit names by dimension.
/// Pass `None` for uncolored output.
pub fn format_unit_list(
    quantity_name: &str,
    unit_names: &[String],
    dims: Option<&crate::units::dimension::DimensionMap>,
    opts: &FormatOptions,
) -> String {
    let t = Theme::new(opts.color);
    let mut lines = Vec::new();

    // Header: quantity name colored by its dimensions.
    let header = match dims {
        Some(d) => t.paint(quantity_name, t.dims_style(d)),
        None => quantity_name.to_string(),
    };
    lines.push(format!("{} ({})", header, unit_names.len()));

    if unit_names.is_empty() {
        lines.push(format!("  {}", t.dim("(no units in database)")));
    } else {
        let styled: Vec<String> = unit_names.iter().map(|n| n.to_string()).collect();
        lines.push(format!("  {}", styled.join(", ")));
    }

    lines.join("\n")
}

/// Format all units grouped by physical quantity.
///
/// Used by `list-units` with no filter. Groups units by their annotation,
/// with an "Other" group for unannotated units.
pub fn format_all_units_grouped(groups: &[(String, Vec<String>)], opts: &FormatOptions) -> String {
    let mut lines = Vec::new();
    for (qty_name, unit_names) in groups {
        lines.push(format_unit_list(qty_name, unit_names, None, opts));
    }
    lines.join("\n\n")
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

    // ---- Explain tests ----

    #[test]
    fn explain_simple_linear_skips_trivial_target_line() {
        // 10 ft → m: target is the base unit, so only `source:` appears.
        let db = UnitDatabase::new();
        let r = run_conversion("10 ft", "m", &db).unwrap();
        let opts = FormatOptions {
            explain: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(out.contains("source:"));
        assert!(!out.contains("target:")); // meter is base — skipped
        assert!(out.contains("0.3048"));
        assert!(out.contains("3.048"));
        // Calculation step: `10 * 0.3048 = 3.048 meter`
        assert!(out.contains("10 * 0.3048"));
    }

    #[test]
    fn explain_compound_linear_shows_both_sides() {
        let db = UnitDatabase::new();
        let r = run_conversion("100 km/h", "mph", &db).unwrap();
        let opts = FormatOptions {
            explain: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(out.contains("source:"));
        assert!(out.contains("target:"));
        assert!(out.contains("62.1371"));
        // Two-step calculation: to base then from base
        assert!(out.contains("0.277778"));
        assert!(out.contains("0.44704"));
    }

    #[test]
    fn explain_affine_uses_same_labels_as_linear() {
        // Harmonization check: affine conversions use `source:` / `target:`
        // labels, not the old `to base:` / `from base:`.
        let db = UnitDatabase::new();
        let r = run_conversion("98.6 degF", "degC", &db).unwrap();
        let opts = FormatOptions {
            explain: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(out.contains("source:"));
        assert!(out.contains("target:"));
        assert!(!out.contains("to base:"));
        assert!(!out.contains("from base:"));
        // Affine formula has the `+ offset` piece
        assert!(out.contains("+ 255.372"));
        assert!(out.contains("37"));
    }

    #[test]
    fn explain_base_to_base_is_identity() {
        // Both sides are the base unit — only the header + standout `= value`.
        let db = UnitDatabase::new();
        let r = run_conversion("5 m", "m", &db).unwrap();
        let opts = FormatOptions {
            explain: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(!out.contains("source:"));
        assert!(!out.contains("target:"));
        assert!(out.contains("= 5 meter"));
    }

    #[test]
    fn explain_has_blank_separator_line() {
        // The calculation section should be preceded by a blank line so it
        // stands out visually.
        let db = UnitDatabase::new();
        let r = run_conversion("10 ft", "m", &db).unwrap();
        let opts = FormatOptions {
            explain: true,
            ..Default::default()
        };
        let out = format_result(&r, &opts);
        assert!(out.contains("\n\n"));
    }

    #[test]
    fn glyphs_ascii_and_unicode_differ() {
        // Sanity check that pick() returns different instances.
        assert_eq!(Glyphs::pick(false).arrow, "->");
        assert_eq!(Glyphs::pick(true).arrow, "\u{2192}");
        assert_eq!(Glyphs::pick(false).times, "*");
        assert_eq!(Glyphs::pick(true).times, "\u{00D7}");
    }
}
