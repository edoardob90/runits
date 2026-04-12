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

// ---------------------------------------------------------------------------
// Quantity arithmetic — inherent methods (not `std::ops::*` trait impls).
//
// Rationale: every arithmetic op on a Quantity can fail (affine unit in an
// expression, dimension mismatch on +/-, even `*`/`/` reject affine), so the
// return type has to be `Result<Quantity, RUnitsError>`. Implementing
// `std::ops::Add::Output = Result<...>` would force `?` at every call site
// AND give up the infix sugar anyway, so there's no upside. Numbat makes the
// same choice. If a future caller ever wants an infallible `Mul` / `Div` impl
// for non-affine cases, add it then — the inherent methods below will still
// win for direct calls.
//
// `clippy::should_implement_trait` is allowed because the method names
// deliberately shadow the trait names; users call these directly from the
// evaluator and should see the fallible shape.
// ---------------------------------------------------------------------------

#[allow(clippy::should_implement_trait)]
impl Quantity {
    /// Add two quantities, returning the result in the **left-hand side's unit**.
    ///
    /// This is the canonical fallible-arithmetic pattern for `Quantity`:
    /// every other binary arithmetic method (`try_sub`, `mul`, `div`,
    /// `pow_i32`) is a variation on this shape. Three decisions are baked
    /// in and are not accidental:
    ///
    /// **1. Error specificity order: affine-reject BEFORE dim-check.** If a
    /// user writes `20 celsius + 5 second`, both the affine check and the
    /// dimension check would fire — but "affine units don't support `+` in
    /// expressions" is a *more specific* and more helpful error than
    /// "Temperature + Time isn't compatible". Most-specific-wins is the
    /// general rule; we order the guards accordingly.
    ///
    /// **2. Always return self's unit, coerce rhs.** `5 m + 3 ft → 5.9144 m`,
    /// not `5.9144 ft`. Left-hand-side is the "preferred" unit. This matches
    /// GNU Units, Numbat, and user intuition — they typed the LHS first,
    /// they probably want the result in those terms.
    ///
    /// **3. Preserve the `?` on `convert_to` even after `is_compatible_with`
    /// passes.** The `?` is technically redundant today — compatibility
    /// implies convertibility — but it documents intent ("I believe this is
    /// safe, but I propagate rather than `.unwrap()`") and is robust against
    /// future changes to compatibility-check semantics (e.g. when we add
    /// nonlinear conversions in Phase 5c, a "compatible" unit pair may still
    /// fail to convert for domain reasons).
    ///
    /// `try_sub` is a near-copy with `op: '-'` and subtraction; kept separate
    /// rather than a shared helper because the match-site clarity is worth
    /// more than the three saved lines.
    pub fn try_add(self, rhs: Quantity) -> Result<Quantity, RUnitsError> {
        if self.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: self.unit.name.clone(),
                op_context: "+".to_string(),
            });
        }
        if rhs.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: rhs.unit.name.clone(),
                op_context: "+".to_string(),
            });
        }
        if !self.unit.is_compatible_with(&rhs.unit) {
            return Err(RUnitsError::IncompatibleAddition {
                op: '+',
                lhs_unit: self.unit.name.clone(),
                rhs_unit: rhs.unit.name.clone(),
                lhs_dim: self.unit.dimension_string(),
                rhs_dim: rhs.unit.dimension_string(),
            });
        }
        let rhs_in_self_unit = rhs.convert_to(&self.unit)?;
        Ok(Quantity::new(
            self.value + rhs_in_self_unit.value,
            self.unit,
        ))
    }

    /// Subtract `rhs` from `self`, returning the result in `self`'s unit.
    ///
    /// See [`try_add`](Self::try_add) for the rationale — this is the same
    /// pattern with `op: '-'`.
    pub fn try_sub(self, rhs: Quantity) -> Result<Quantity, RUnitsError> {
        if self.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: self.unit.name.clone(),
                op_context: "-".to_string(),
            });
        }
        if rhs.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: rhs.unit.name.clone(),
                op_context: "-".to_string(),
            });
        }
        if !self.unit.is_compatible_with(&rhs.unit) {
            return Err(RUnitsError::IncompatibleAddition {
                op: '-',
                lhs_unit: self.unit.name.clone(),
                rhs_unit: rhs.unit.name.clone(),
                lhs_dim: self.unit.dimension_string(),
                rhs_dim: rhs.unit.dimension_string(),
            });
        }
        let rhs_in_self_unit = rhs.convert_to(&self.unit)?;
        Ok(Quantity::new(
            self.value - rhs_in_self_unit.value,
            self.unit,
        ))
    }

    /// Multiply two quantities.
    ///
    /// Three paths, in order:
    ///
    /// 1. **Scalar × bare affine** — the natural user input `"98.6 degF"`
    ///    arrives here as `Number(98.6) * Ident("degF")`, i.e. a dimensionless
    ///    scalar multiplied by a `Quantity{ value: 1.0, unit: degF }`. Treat
    ///    it as "apply the scalar value to the affine unit" and return
    ///    `Quantity{ 98.6, degF }`. The affine side must still be the bare
    ///    unit reference (`value == 1.0`); `2 * 98.6 degF` is rejected
    ///    because scaling a temperature is meaningless.
    ///
    /// 2. **Scalar × non-affine unit** — e.g. `10 m`. Keep the unit name
    ///    clean ("meter", not "dimensionless*meter") by promoting the
    ///    scalar's value directly into the other side. Rhs-scalar case
    ///    (`m * 10`) is handled symmetrically. Dividing by a pure number
    ///    in `div` has the mirror treatment.
    ///
    /// 3. **Full compound** — both sides carry dimensions. Fall through to
    ///    the existing `Unit * Unit` impl, which multiplies conversion
    ///    factors and sums dimension exponents. Rejects affine.
    ///
    /// "Scalar" here means "dimensionless, non-affine" — any unit with an
    /// empty dimension map.
    pub fn mul(self, rhs: Quantity) -> Result<Quantity, RUnitsError> {
        let lhs_is_scalar = self.unit.dimensions.is_empty() && !self.unit.is_affine();
        let rhs_is_scalar = rhs.unit.dimensions.is_empty() && !rhs.unit.is_affine();

        // (1) Scalar × affine unit (and mirror).
        if lhs_is_scalar && rhs.unit.is_affine() {
            if (rhs.value - 1.0).abs() < 1e-12 {
                return Ok(Quantity::new(self.value, rhs.unit));
            }
            return Err(RUnitsError::AffineInExpression {
                unit: rhs.unit.name.clone(),
                op_context: "*".to_string(),
            });
        }
        if rhs_is_scalar && self.unit.is_affine() {
            if (self.value - 1.0).abs() < 1e-12 {
                return Ok(Quantity::new(rhs.value, self.unit));
            }
            return Err(RUnitsError::AffineInExpression {
                unit: self.unit.name.clone(),
                op_context: "*".to_string(),
            });
        }

        if self.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: self.unit.name.clone(),
                op_context: "*".to_string(),
            });
        }
        if rhs.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: rhs.unit.name.clone(),
                op_context: "*".to_string(),
            });
        }

        // (2) Scalar × non-affine unit: clean-unit fast path.
        if lhs_is_scalar {
            return Ok(Quantity::new(self.value * rhs.value, rhs.unit));
        }
        if rhs_is_scalar {
            return Ok(Quantity::new(self.value * rhs.value, self.unit));
        }

        // (3) Full compound multiplication.
        let value = self.value * rhs.value;
        let unit = self.unit * rhs.unit;
        Ok(Quantity::new(value, unit))
    }

    /// Divide two quantities. Rejects affine units on either side.
    ///
    /// Divide-by-scalar is short-circuited to keep the unit name clean
    /// (e.g. `10 m / 2` stays `5 m`, not `5 m/dimensionless`). The mirror
    /// case — scalar divided by a unit — falls through to `Unit / Unit`,
    /// which yields `dimensionless/m` as a name but with the correct
    /// `Length^-1` dimensions; that path is rare enough not to warrant its
    /// own fast lane.
    pub fn div(self, rhs: Quantity) -> Result<Quantity, RUnitsError> {
        if self.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: self.unit.name.clone(),
                op_context: "/".to_string(),
            });
        }
        if rhs.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: rhs.unit.name.clone(),
                op_context: "/".to_string(),
            });
        }

        let rhs_is_scalar = rhs.unit.dimensions.is_empty() && !rhs.unit.is_affine();
        if rhs_is_scalar {
            return Ok(Quantity::new(self.value / rhs.value, self.unit));
        }

        let value = self.value / rhs.value;
        let unit = self.unit / rhs.unit;
        Ok(Quantity::new(value, unit))
    }

    /// Negate this quantity in place (value negated, unit unchanged).
    ///
    /// Takes `self` by value (not `&self`) so the caller's `Unit` — which
    /// owns a `String` name and a `HashMap` of dimensions — can be moved
    /// into the result without cloning. A `&self` version would force a
    /// `.clone()` of the unit on every negation. Every `Quantity` arithmetic
    /// method in this module follows the same by-value convention for the
    /// same reason; changing this convention in one method without changing
    /// it everywhere would create allocation cliffs in arithmetic expressions.
    pub fn neg(self) -> Quantity {
        Quantity::new(-self.value, self.unit)
    }

    /// Raise a quantity to an integer power. Rejects affine units.
    ///
    /// Unit and value are both raised: the unit via [`pow_unit`](super::unit::pow_unit),
    /// the value via `f64::powi`. Exponent 0 yields dimensionless `1.0`;
    /// negative exponents invert first (via `pow_unit`'s recursion).
    pub fn pow_i32(self, exp: i32) -> Result<Quantity, RUnitsError> {
        if self.unit.is_affine() {
            return Err(RUnitsError::AffineInExpression {
                unit: self.unit.name.clone(),
                op_context: "^".to_string(),
            });
        }
        let value = self.value.powi(exp);
        let unit = super::unit::pow_unit(self.unit, exp);
        Ok(Quantity::new(value, unit))
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

pub fn format_value_inner(
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
        assert!(msg.contains("Length") && msg.contains("Time"));
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
        assert_eq!(format_value(1.23456789, 4, false), "1.235");
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

    // ---- Quantity arithmetic (Phase 5a) ----

    #[test]
    fn try_add_same_unit() {
        let a = Quantity::new(5.0, Unit::meter());
        let b = Quantity::new(3.0, Unit::meter());
        let sum = a.try_add(b).unwrap();
        assert!((sum.value - 8.0).abs() < 1e-12);
        assert_eq!(sum.unit.name, "meter");
    }

    #[test]
    fn try_add_compatible_units_coerces_rhs() {
        // 5 m + 3 ft → should stay in meters, add the converted ft value
        let a = Quantity::new(5.0, Unit::meter());
        let b = Quantity::new(3.0, Unit::foot());
        let sum = a.try_add(b).unwrap();
        // 3 ft = 0.9144 m; total = 5.9144 m
        assert!((sum.value - 5.9144).abs() < 1e-9);
        assert_eq!(sum.unit.name, "meter");
    }

    #[test]
    fn try_add_incompatible_fails() {
        let a = Quantity::new(5.0, Unit::meter());
        let b = Quantity::new(3.0, Unit::second());
        let err = a.try_add(b).unwrap_err();
        match err {
            RUnitsError::IncompatibleAddition {
                op,
                lhs_dim,
                rhs_dim,
                ..
            } => {
                assert_eq!(op, '+');
                assert!(lhs_dim.contains("Length"));
                assert!(rhs_dim.contains("Time"));
            }
            other => panic!("expected IncompatibleAddition, got {other:?}"),
        }
    }

    #[test]
    fn try_add_affine_fails_before_dim_check() {
        // Even if dimensions match, affine must be rejected first with
        // the more-specific error.
        let a = Quantity::new(20.0, Unit::celsius());
        let b = Quantity::new(5.0, Unit::celsius());
        let err = a.try_add(b).unwrap_err();
        assert!(matches!(err, RUnitsError::AffineInExpression { .. }));
    }

    #[test]
    fn try_sub_same_unit() {
        let a = Quantity::new(10.0, Unit::meter());
        let b = Quantity::new(3.0, Unit::meter());
        let diff = a.try_sub(b).unwrap();
        assert!((diff.value - 7.0).abs() < 1e-12);
    }

    #[test]
    fn mul_multiplies_values_and_units() {
        let a = Quantity::new(5.0, Unit::meter());
        let b = Quantity::new(3.0, Unit::meter());
        let prod = a.mul(b).unwrap();
        assert!((prod.value - 15.0).abs() < 1e-12);
        assert_eq!(prod.unit.dimension_string(), "Length^2");
    }

    #[test]
    fn div_divides_values_and_units() {
        let a = Quantity::new(10.0, Unit::meter());
        let b = Quantity::new(2.0, Unit::second());
        let result = a.div(b).unwrap();
        assert!((result.value - 5.0).abs() < 1e-12);
        assert_eq!(result.unit.dimension_string(), "Length*Time^-1");
    }

    #[test]
    fn mul_rejects_affine() {
        let a = Quantity::new(20.0, Unit::celsius());
        let b = Quantity::new(5.0, Unit::meter());
        let err = a.mul(b).unwrap_err();
        assert!(matches!(err, RUnitsError::AffineInExpression { .. }));
    }

    #[test]
    fn neg_preserves_unit() {
        let q = Quantity::new(5.0, Unit::meter());
        let n = q.neg();
        assert!((n.value + 5.0).abs() < 1e-12);
        assert_eq!(n.unit.name, "meter");
    }

    #[test]
    fn pow_i32_basic() {
        let q = Quantity::new(3.0, Unit::meter());
        let sq = q.pow_i32(2).unwrap();
        assert!((sq.value - 9.0).abs() < 1e-12);
        assert_eq!(sq.unit.dimension_string(), "Length^2");
    }

    #[test]
    fn pow_i32_zero_yields_dimensionless() {
        let q = Quantity::new(5.0, Unit::meter());
        let result = q.pow_i32(0).unwrap();
        assert!((result.value - 1.0).abs() < 1e-12);
        assert!(result.unit.dimensions.is_empty());
    }

    #[test]
    fn pow_i32_negative_inverts() {
        let q = Quantity::new(2.0, Unit::meter());
        let inv = q.pow_i32(-1).unwrap();
        assert!((inv.value - 0.5).abs() < 1e-12);
        assert_eq!(inv.unit.dimension_string(), "Length^-1");
    }

    #[test]
    fn pow_i32_affine_fails() {
        let q = Quantity::new(20.0, Unit::celsius());
        let err = q.pow_i32(2).unwrap_err();
        assert!(matches!(err, RUnitsError::AffineInExpression { .. }));
    }
}
