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

        // Convert through base units: value * from_factor / to_factor.
        let base_value = self.value * self.unit.conversion_factor;
        let target_value = base_value / target_unit.conversion_factor;

        Ok(Quantity::new(target_value, target_unit.clone()))
    }

    /// Like [`convert_to`](Self::convert_to) but returns just the numeric value.
    pub fn convert_value_to(&self, target_unit: &Unit) -> Result<f64, RUnitsError> {
        self.convert_to(target_unit).map(|q| q.value)
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", format_value(self.value), self.unit.name)
    }
}

/// Format an `f64` for human display with 6 significant figures.
///
/// Rounding to 6 sig figs removes floating-point representation noise
/// (`0.000049999999999999996` → `0.00005`). The output switches between
/// decimal and scientific notation based on magnitude so extreme values
/// don't bloat into long strings of digits.
///
/// - Decimal when `1e-4 ≤ |v| < 1e7`
/// - Scientific (`{:e}`) outside that band
/// - Plain passthrough for zero / NaN / infinity
///
/// Phase 3 will add a user-configurable precision flag; this function
/// is where that knob will plug in.
fn format_value(v: f64) -> String {
    if v == 0.0 || v.is_nan() || v.is_infinite() {
        return format!("{}", v);
    }

    let abs = v.abs();
    const SIG_FIGS: usize = 6;

    // Round by round-tripping through `{:.N$e}` — printf-style formatting
    // does exact decimal-digit rounding internally, avoiding the
    // multiply-by-10^k FP drift that plagues manual rounding for very
    // large/small magnitudes. `{:.5e}` gives 6 significant figures
    // (1 digit before the point + 5 after).
    let rounded_str = format!("{:.*e}", SIG_FIGS - 1, v);
    let rounded: f64 = rounded_str
        .parse()
        .expect("round-trip our own scientific-notation output");

    if (1e-4..1e7).contains(&abs) {
        format!("{}", rounded)
    } else {
        format!("{:e}", rounded)
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
        assert!((result.value - 10.0 * unit.conversion_factor).abs() < 0.001);
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
        assert_eq!(format_value(3.048), "3.048");
        assert_eq!(format_value(8.04672), "8.04672");
        assert_eq!(format_value(3600.0), "3600");
        assert_eq!(format_value(0.5), "0.5");
    }

    #[test]
    fn format_value_eliminates_float_noise() {
        // 50e-6 in f64 is 0.000049999999999999996; 6 sig figs rounds it.
        let noisy = 50.0 * 1e-6;
        // Just under 1e-4 → scientific form `5e-5`.
        assert_eq!(format_value(noisy), "5e-5");
    }

    #[test]
    fn format_value_scientific_for_tiny_values() {
        assert_eq!(format_value(5e-5), "5e-5");
        assert_eq!(format_value(1.5e-7), "1.5e-7");
    }

    #[test]
    fn format_value_scientific_for_huge_values() {
        // Avogadro's number round-trip.
        assert_eq!(format_value(6.022e23), "6.022e23");
        assert_eq!(format_value(1e7), "1e7");
    }

    #[test]
    fn format_value_rounds_to_six_sig_figs() {
        // 62.1371192237334 → 62.1371
        assert_eq!(format_value(62.1371192237334), "62.1371");
    }

    #[test]
    fn format_value_handles_zero_and_negatives() {
        assert_eq!(format_value(0.0), "0");
        assert_eq!(format_value(-3.048), "-3.048");
    }

    #[test]
    fn format_value_boundary_1e_minus_4() {
        // 1e-4 is exactly ON the boundary → decimal.
        assert_eq!(format_value(1e-4), "0.0001");
        // Just below boundary → scientific.
        assert_eq!(format_value(9.9e-5), "9.9e-5");
    }
}
