//! The Quantity object: a number paired with a unit.
//!
//! A [`Quantity`] is the core data type users manipulate — `10 meter`, `3.5 foot`,
//! `6.022e23 mole`. It carries the value, the unit, and (via the unit's
//! [`DimensionMap`](crate::units::dimension::DimensionMap)) full dimensional
//! information, enabling safe conversions with runtime dimensional checking.

use super::unit::Unit;
use crate::error::RUnitsError;
use std::fmt;

/// A physical quantity: a numeric value tagged with a [`Unit`].
///
/// `Quantity` is the type returned by parsing user input and the type the
/// CLI ultimately converts and prints. Arithmetic is deferred to the unit
/// itself (see [`Unit`]'s `Mul`/`Div` impls).
#[derive(Debug, Clone)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

impl Quantity {
    /// Constructs a quantity from a value and owned unit.
    pub fn new(value: f64, unit: Unit) -> Self {
        Quantity { value, unit }
    }

    /// Converts this quantity to the given target unit.
    ///
    /// Returns [`RUnitsError::IncompatibleDimensions`] if the target unit's
    /// dimensions don't match this quantity's. The error carries both unit
    /// names and both dimension strings so the CLI can print a useful message.
    pub fn convert_to(&self, target_unit: &Unit) -> Result<Quantity, RUnitsError> {
        if !self.unit.is_compatible_with(target_unit) {
            return Err(RUnitsError::IncompatibleDimensions {
                from: self.unit.name.clone(),
                to: target_unit.name.clone(),
                from_dim: self.unit.dimension_string(),
                to_dim: target_unit.dimension_string(),
            });
        }

        // Convert through base units: source → base → target.
        let base_value = self.unit.to_base_value(self.value);
        let target_value = target_unit.from_base_value(base_value);

        Ok(Quantity::new(target_value, target_unit.clone()))
    }

    /// Like [`convert_to`](Self::convert_to) but returns just the numeric value.
    pub fn convert_value_to(&self, target_unit: &Unit) -> Result<f64, RUnitsError> {
        self.convert_to(target_unit).map(|q| q.value)
    }
}

impl Quantity {
    /// Format this quantity with custom precision and notation.
    ///
    /// Format with explicit flags. Preserves trailing zeros to honor the
    /// user's requested precision. CLI output is pipe-friendly: no annotations.
    pub fn format_with(&self, sig_figs: usize, force_scientific: bool, unit_name: &str) -> String {
        format!(
            "{} {}",
            format_value_inner(self.value, sig_figs, force_scientific, true),
            unit_name
        )
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            format_value(self.value, 6, false),
            self.unit.name
        )
    }
}

/// Format an `f64` for human display with configurable significant figures.
///
/// Trailing zeros are stripped (`3.048` not `3.04800`). The internal
/// `format_with` method on `Quantity` preserves them when the user
/// explicitly sets `--precision`.
///
/// - Decimal when `1e-4 ≤ |v| < 1e7` (and `force_scientific` is false)
/// - Scientific (`{:e}`) outside that band, or always if `force_scientific`
/// - Plain passthrough for zero / NaN / infinity
pub fn format_value(v: f64, sig_figs: usize, force_scientific: bool) -> String {
    format_value_inner(v, sig_figs, force_scientific, false)
}

fn format_value_inner(
    v: f64,
    sig_figs: usize,
    force_scientific: bool,
    exact_sig_figs: bool,
) -> String {
    if v == 0.0 || v.is_nan() || v.is_infinite() {
        return format!("{}", v);
    }

    let abs = v.abs();
    let sig_figs = sig_figs.max(1);

    // Round via printf-style scientific notation — exact decimal-digit rounding
    // internally, no FP drift from manual 10^k scaling.
    let rounded_str = format!("{:.*e}", sig_figs - 1, v);

    if force_scientific || !(1e-4..1e7).contains(&abs) {
        if exact_sig_figs {
            rounded_str
        } else {
            // Strip trailing zeros from mantissa: "3.60000e3" → "3.6e3"
            strip_scientific_zeros(&rounded_str)
        }
    } else {
        let rounded: f64 = rounded_str
            .parse()
            .expect("round-trip our own scientific-notation output");
        if exact_sig_figs {
            let magnitude = if rounded != 0.0 {
                rounded.abs().log10().floor() as i32
            } else {
                0
            };
            let decimal_places = (sig_figs as i32 - 1 - magnitude).max(0) as usize;
            format!("{:.*}", decimal_places, rounded)
        } else {
            format!("{}", rounded)
        }
    }
}

/// Strip trailing zeros from scientific notation: "3.60000e3" → "3.6e3", "5.00000e-5" → "5e-5"
fn strip_scientific_zeros(s: &str) -> String {
    if let Some(e_pos) = s.find('e') {
        let mantissa = &s[..e_pos];
        let exponent = &s[e_pos..];
        let trimmed = mantissa.trim_end_matches('0').trim_end_matches('.');
        format!("{}{}", trimmed, exponent)
    } else {
        s.to_string()
    }
}

/// Standalone helper for when you don't already have a `Quantity` in hand.
pub fn convert_quantity(value: f64, from_unit: &Unit, to_unit: &Unit) -> Result<f64, RUnitsError> {
    let quantity = Quantity::new(value, from_unit.clone());
    quantity.convert_value_to(to_unit)
}

// Convenience factories for the most common quantity shapes.
impl Quantity {
    pub fn meters(value: f64) -> Self {
        Self::new(value, Unit::meter())
    }

    pub fn feet(value: f64) -> Self {
        Self::new(value, Unit::foot())
    }

    pub fn seconds(value: f64) -> Self {
        Self::new(value, Unit::second())
    }

    pub fn kilograms(value: f64) -> Self {
        Self::new(value, Unit::kilogram())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shorthand for format_value with default args (6 sig figs, no forced scientific).
    fn fv(v: f64) -> String {
        format_value(v, 6, false)
    }

    #[test]
    fn test_quantity_creation() {
        let distance = Quantity::meters(10.0);
        assert_eq!(distance.value, 10.0);
        assert_eq!(distance.unit.name, "meter");
    }

    #[test]
    fn test_successful_conversion() {
        let unit = Unit::foot();
        let q = Quantity::new(10.0, unit.clone());
        let result = q.convert_to(&Unit::meter()).unwrap();
        assert!((result.value - 10.0 * unit.conversion_factor()).abs() < 0.001);
    }

    #[test]
    fn test_failed_conversion_reports_dimensions() {
        let q = Quantity::new(1.0, Unit::meter());
        let err = q.convert_to(&Unit::second()).unwrap_err();
        let msg = err.to_string();
        // Error message should mention both units AND both dimensions,
        // so the user immediately sees why the conversion is illegal.
        assert!(msg.contains("meter") && msg.contains("second"));
        assert!(msg.contains("length") && msg.contains("time"));
    }

    #[test]
    fn test_round_trip_conversion() {
        let q1 = Quantity::new(5.0, Unit::meter());
        let q2 = q1.convert_to(&Unit::foot()).unwrap();
        let q3 = q2.convert_to(&Unit::meter()).unwrap();
        assert!((q1.value - q3.value).abs() < 0.001);
    }

    #[test]
    fn test_display_trait() {
        let distance = Quantity::meters(10.5);
        let display_string = format!("{}", distance);
        assert_eq!(display_string, "10.5 meter");
    }

    #[test]
    fn format_value_decimal_in_band() {
        assert_eq!(fv(3.048), "3.048");
        assert_eq!(fv(8.04672), "8.04672");
        assert_eq!(fv(3600.0), "3600");
        assert_eq!(fv(0.5), "0.5");
    }

    #[test]
    fn format_value_eliminates_float_noise() {
        let noisy = 50.0 * 1e-6;
        assert_eq!(fv(noisy), "5e-5");
    }

    #[test]
    fn format_value_scientific_for_tiny_values() {
        assert_eq!(fv(5e-5), "5e-5");
        assert_eq!(fv(1.5e-7), "1.5e-7");
    }

    #[test]
    fn format_value_scientific_for_huge_values() {
        assert_eq!(fv(6.022e23), "6.022e23");
        assert_eq!(fv(1e7), "1e7");
    }

    #[test]
    fn format_value_rounds_to_six_sig_figs() {
        assert_eq!(fv(62.1371192237334), "62.1371");
    }

    #[test]
    fn format_value_handles_zero_and_negatives() {
        assert_eq!(fv(0.0), "0");
        assert_eq!(fv(-3.048), "-3.048");
    }

    #[test]
    fn format_value_boundary_1e_minus_4() {
        assert_eq!(fv(1e-4), "0.0001");
        assert_eq!(fv(9.9e-5), "9.9e-5");
    }

    #[test]
    fn format_value_custom_precision() {
        // Default: strips trailing zeros
        assert_eq!(format_value(62.1371192237334, 3, false), "62.1");
        assert_eq!(format_value(3.14159265, 4, false), "3.142");
    }

    #[test]
    fn format_value_force_scientific() {
        // Default: strips trailing zeros in scientific mode too
        assert_eq!(format_value(3600.0, 6, true), "3.6e3");
        assert_eq!(format_value(0.5, 6, true), "5e-1");
    }

    #[test]
    fn format_value_exact_preserves_trailing_zeros() {
        // Explicit precision: trailing zeros preserved
        assert_eq!(format_value_inner(3.048, 10, false, true), "3.048000000");
        assert_eq!(format_value_inner(3600.0, 6, true, true), "3.60000e3");
    }
}
