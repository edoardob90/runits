//! Math function registry — foundation for a Numbat-style math prelude.
//!
//! ## Design: enum + exhaustive match, not trait objects
//!
//! Rationale:
//! - **Exhaustiveness wins.** With ~30 functions in the future, adding
//!   `MathFn::Log10` and forgetting to wire up a signature becomes a compile
//!   error, not a silent default.
//! - **Metadata is data, not behavior.** Arity, domain constraints, display
//!   names are better as `const` tables near the enum than as methods spread
//!   across N structs.
//! - **Adding a function = edit one file in four places** (enum variant,
//!   `name` arm, `apply` arm, `ALL` slice entry) — no `Box`, no `HashMap`, no
//!   registration ceremony.
//! - **Trait objects deferred to a better learning context.** `Box<dyn Trait>`
//!   is worth teaching where dynamic dispatch actually earns its keep — e.g.,
//!   a future `Reader` trait for unit database sources, or a `Formatter`
//!   trait for output styles. A function table with ~30 known-at-compile-time
//!   variants isn't that context.
//!
//! ## Extension recipe
//!
//! 1. Add a variant to `MathFn`: e.g. `MathFn::Log10`.
//! 2. Add matching arms in `name`, `signature`, `apply`, and add the variant
//!    to `MathFn::ALL`.
//! 3. For a scalar function, `apply`'s arm calls `apply_scalar(self.name(),
//!    &args[0], f64::log10)`. For a dimension-transforming function, copy the
//!    shape of `apply_sqrt` and validate dimensions before halving/doubling.
//! 4. Write a unit test.
//!
//! No grammar change, no evaluator change, no `Box`, no registration — the
//! compiler will flag any arm you forgot.

use crate::error::RUnitsError;
use crate::units::dimension::Dimension;
use crate::units::{Quantity, Unit};

/// The set of built-in math functions.
///
/// Covers all three dimensionality categories (following Numbat's
/// classification):
///
/// - **Dimension-transforming:** `sqrt(D²) → D`, `sqr(D) → D²`
/// - **Dimension-generic:** `abs(D) → D`
/// - **Scalar-only (transcendental / trig):** `sin`, `cos`, `tan`, `ln`, `exp`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathFn {
    // Dimension-transforming
    Sqrt,
    Sqr,
    // Dimension-generic
    Abs,
    // Scalar-only — transcendental + trig
    Sin,
    Cos,
    Tan,
    Ln,
    Exp,
}

/// How many arguments a function accepts.
///
/// Kept as an enum (rather than a `usize`) so `Exact(1)` reads clearly at
/// call sites, and so future `AtLeast(n)` / `Range(min, max)` variants have
/// a place to land if variadic functions (like `max(a, b, c, ...)`) arrive.
#[derive(Debug, Clone, Copy)]
pub enum Arity {
    Exact(usize),
}

impl MathFn {
    /// Every built-in math function, in declaration order.
    ///
    /// Used by `lookup`, `suggest`, and the completions helper.
    pub const ALL: &'static [MathFn] = &[
        MathFn::Sqrt,
        MathFn::Sqr,
        MathFn::Abs,
        MathFn::Sin,
        MathFn::Cos,
        MathFn::Tan,
        MathFn::Ln,
        MathFn::Exp,
    ];

    /// The function's name as it appears in user input.
    pub fn name(self) -> &'static str {
        match self {
            MathFn::Sqrt => "sqrt",
            MathFn::Sqr => "sqr",
            MathFn::Abs => "abs",
            MathFn::Sin => "sin",
            MathFn::Cos => "cos",
            MathFn::Tan => "tan",
            MathFn::Ln => "ln",
            MathFn::Exp => "exp",
        }
    }

    /// Human-readable signature for `?`-help output.
    pub fn signature(self) -> &'static str {
        match self {
            MathFn::Sqrt => "sqrt<D>(D²) → D",
            MathFn::Sqr => "sqr<D>(D) → D²",
            MathFn::Abs => "abs<D>(D) → D",
            MathFn::Sin | MathFn::Cos | MathFn::Tan => "(Scalar) → Scalar",
            MathFn::Ln | MathFn::Exp => "(Scalar) → Scalar",
        }
    }

    /// Number of arguments the function accepts.
    ///
    /// All initial variants are unary; widening this to support e.g.
    /// `atan2(y, x)` means changing this method and the arity check in
    /// `apply`, nothing else.
    pub fn arity(self) -> Arity {
        Arity::Exact(1)
    }

    /// Evaluate the function on a slice of quantity arguments.
    ///
    /// Checks arity once at the top, then dispatches to the per-function
    /// implementation. Per-function errors (domain violations, affine
    /// rejection, etc.) are produced inside each `apply_*` helper.
    pub fn apply(self, args: &[Quantity]) -> Result<Quantity, RUnitsError> {
        let Arity::Exact(n) = self.arity();
        if args.len() != n {
            return Err(RUnitsError::ArityMismatch {
                name: self.name(),
                expected: n,
                got: args.len(),
            });
        }
        match self {
            MathFn::Sqrt => apply_sqrt(&args[0]),
            MathFn::Sqr => apply_sqr(&args[0]),
            MathFn::Abs => apply_abs(&args[0]),
            MathFn::Sin => apply_scalar(self.name(), &args[0], f64::sin),
            MathFn::Cos => apply_scalar(self.name(), &args[0], f64::cos),
            MathFn::Tan => apply_scalar(self.name(), &args[0], f64::tan),
            MathFn::Ln => apply_scalar(self.name(), &args[0], f64::ln),
            MathFn::Exp => apply_scalar(self.name(), &args[0], f64::exp),
        }
    }
}

/// Look up a math function by its user-facing name.
pub fn lookup(name: &str) -> Option<MathFn> {
    MathFn::ALL.iter().copied().find(|f| f.name() == name)
}

/// Suggest the closest math function names for a misspelled input.
///
/// Uses Jaro-Winkler similarity like the unit database, with the same
/// `> 0.7` score threshold. Returns up to `max` candidates.
pub fn suggest(unknown: &str, max: usize) -> Vec<String> {
    let unknown_lower = unknown.to_lowercase();
    let mut scored: Vec<_> = MathFn::ALL
        .iter()
        .map(|f| {
            let name = f.name();
            let score = strsim::jaro_winkler(&unknown_lower, &name.to_lowercase());
            (name, score)
        })
        .filter(|(_, score)| *score > 0.7)
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored
        .into_iter()
        .take(max)
        .map(|(name, _)| name.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Per-function helpers. Each produces a `FunctionDomainError` (or the shared
// `AffineInExpression`) for inputs it can't handle.
// ---------------------------------------------------------------------------

/// Reject affine-unit arguments up front. Shared with every `apply_*` helper
/// so the error path is uniform.
fn require_non_affine(q: &Quantity, fn_name: &'static str) -> Result<(), RUnitsError> {
    if q.unit.is_affine() {
        Err(RUnitsError::AffineInExpression {
            unit: q.unit.name.clone(),
            op_context: fn_name.to_string(),
        })
    } else {
        Ok(())
    }
}

/// Square-root of a dimensioned quantity requires both a value sqrt AND a
/// halving of every dimension exponent. The non-obvious part is how the
/// unit's `conversion_factor` interacts with both.
///
/// **Why the base-unit trip matters.** Consider `sqrt(9 km²)`:
///   - `q.value = 9`, `q.unit.conversion_factor = 10⁶` (1 km² = 10⁶ m²),
///     `dimensions = {Length: 2}`.
///   - Wrong approach: `sqrt(9) = 3`, halve dimensions to `{Length: 1}`,
///     build a unit with factor 1.0. You get `3 m` — but the right answer is
///     `3 km = 3000 m`.
///   - Right approach: convert to base first. `9` in km² becomes
///     `9 × 10⁶ = 9,000,000` in m². `sqrt(9,000,000) = 3000` in m. Build a
///     base-factor (1.0) unit with halved dims. Wrap and return: `3000 m`.
///     Correct.
///
/// The "wrong" approach forgets that the conversion factor also needs its
/// square root. The base-unit trip side-steps this by never having to reason
/// about sqrt-of-factor at all — we express the answer in base dimensions
/// with factor 1.0.
///
/// **Why validate before halving.** If any exponent is odd, we need to error
/// before building the halved vector: producing a vec with fractional
/// exponents would silently round (i8 division) and return a meaningless
/// unit. Validation-before-construction is an invariant every dimension-
/// transforming function in this module should follow.
///
/// **Why `format!("sqrt({})", q.unit.name)` for the unit name.** Display-only
/// placeholder. Proper compound-name simplification (turning
/// `"sqrt(meter*meter)"` into `"meter"`) is a separate Extras-Catalog item in
/// the roadmap. Until then, the name is a bit ugly but correct — and the
/// `--explain` output always renders from dimensions anyway.
///
/// This function is the template for every future dimension-transforming
/// function (`cbrt`, `hypot2`, `quadratic_equation`, ...). Copy its shape.
fn apply_sqrt(q: &Quantity) -> Result<Quantity, RUnitsError> {
    require_non_affine(q, "sqrt")?;
    if q.value < 0.0 {
        return Err(RUnitsError::FunctionDomainError {
            name: "sqrt",
            reason: format!("negative argument ({})", q.value),
        });
    }

    // Part A: every dimension exponent must be even.
    for (dim, &exp) in q.unit.dimensions.iter() {
        if exp % 2 != 0 {
            return Err(RUnitsError::FunctionDomainError {
                name: "sqrt",
                reason: format!(
                    "dimension {} has odd exponent {}; sqrt requires perfect-square dimensions",
                    dim.name(),
                    exp
                ),
            });
        }
    }

    // Part B: halved dimension slice.
    let halved_dims: Vec<(Dimension, i8)> = q
        .unit
        .dimensions
        .iter()
        .map(|(dim, &exp)| (dim.clone(), exp / 2))
        .collect();

    // Part C: base-unit trip — convert value to base, sqrt, re-express.
    let base_value = q.unit.to_base_value(q.value);
    let sqrt_value = base_value.sqrt();

    // Part D: new unit in base factor (1.0) with halved dims.
    let new_unit = Unit::new(&format!("sqrt({})", q.unit.name), 1.0, &halved_dims);

    Ok(Quantity::new(sqrt_value, new_unit))
}

/// `sqr(x) = x * x`, with dimensions doubled.
///
/// Mirrors `apply_sqrt`'s shape but in the other direction: the base-unit
/// trip avoids having to square the unit's conversion_factor, and the
/// doubled dimensions are always valid (no parity check needed).
fn apply_sqr(q: &Quantity) -> Result<Quantity, RUnitsError> {
    require_non_affine(q, "sqr")?;

    let doubled_dims: Vec<(Dimension, i8)> = q
        .unit
        .dimensions
        .iter()
        .map(|(dim, &exp)| (dim.clone(), exp * 2))
        .collect();

    let base_value = q.unit.to_base_value(q.value);
    let sqr_value = base_value * base_value;

    let new_unit = Unit::new(&format!("sqr({})", q.unit.name), 1.0, &doubled_dims);
    Ok(Quantity::new(sqr_value, new_unit))
}

/// `abs(x)` — preserves unit, flips sign of the value if negative.
///
/// Dimension-generic: any non-affine quantity is valid. We keep the original
/// unit (name, factor, dimensions) rather than routing through base, because
/// there's no dimension math to do — the unit is unchanged.
fn apply_abs(q: &Quantity) -> Result<Quantity, RUnitsError> {
    require_non_affine(q, "abs")?;
    Ok(Quantity::new(q.value.abs(), q.unit.clone()))
}

/// Scalar-only math function: require a dimensionless argument, then apply
/// a plain `f64 -> f64` function. Shared between `sin`, `cos`, `tan`, `ln`,
/// `exp`, and any future transcendental that obeys the same contract.
fn apply_scalar(
    name: &'static str,
    q: &Quantity,
    f: fn(f64) -> f64,
) -> Result<Quantity, RUnitsError> {
    require_non_affine(q, name)?;
    if !q.unit.dimensions.is_empty() {
        return Err(RUnitsError::FunctionDomainError {
            name,
            reason: format!(
                "expects dimensionless argument (got {})",
                q.unit.dimension_string()
            ),
        });
    }
    Ok(Quantity::new(f(q.value), Unit::dimensionless()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_all_variants() {
        for f in MathFn::ALL {
            assert_eq!(lookup(f.name()), Some(*f));
        }
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup("nope").is_none());
    }

    #[test]
    fn suggest_typo_returns_close_match() {
        let s = suggest("sxrt", 3);
        assert!(s.contains(&"sqrt".to_string()));
    }

    #[test]
    fn suggest_gibberish_returns_empty() {
        let s = suggest("xyzzy", 3);
        assert!(s.is_empty());
    }

    #[test]
    fn sqrt_of_perfect_square_dimensions() {
        let q = Quantity::new(9.0, Unit::new("meter^2", 1.0, &[(Dimension::Length, 2)]));
        let r = MathFn::Sqrt.apply(&[q]).unwrap();
        assert!((r.value - 3.0).abs() < 1e-12);
        assert_eq!(r.unit.dimension_string(), "Length");
    }

    #[test]
    fn sqrt_base_trip_km_squared() {
        // 9 (km^2) should produce 3 km = 3000 m in base.
        let km_sq = Unit::new("km^2", 1_000_000.0, &[(Dimension::Length, 2)]);
        let q = Quantity::new(9.0, km_sq);
        let r = MathFn::Sqrt.apply(&[q]).unwrap();
        // Result is in base (factor 1.0), so value should be 3000.
        assert!((r.value - 3000.0).abs() < 1e-9);
        assert_eq!(r.unit.dimension_string(), "Length");
    }

    #[test]
    fn sqrt_odd_exponent_fails() {
        let q = Quantity::new(9.0, Unit::meter());
        let err = MathFn::Sqrt.apply(&[q]).unwrap_err();
        assert!(matches!(err, RUnitsError::FunctionDomainError { .. }));
    }

    #[test]
    fn sqrt_negative_fails() {
        let q = Quantity::new(-9.0, Unit::new("meter^2", 1.0, &[(Dimension::Length, 2)]));
        let err = MathFn::Sqrt.apply(&[q]).unwrap_err();
        assert!(matches!(err, RUnitsError::FunctionDomainError { .. }));
    }

    #[test]
    fn sqr_doubles_dimensions() {
        let q = Quantity::new(3.0, Unit::meter());
        let r = MathFn::Sqr.apply(&[q]).unwrap();
        assert!((r.value - 9.0).abs() < 1e-12);
        assert_eq!(r.unit.dimension_string(), "Length^2");
    }

    #[test]
    fn abs_preserves_unit() {
        let q = Quantity::new(-5.0, Unit::meter());
        let r = MathFn::Abs.apply(&[q]).unwrap();
        assert!((r.value - 5.0).abs() < 1e-12);
        assert_eq!(r.unit.name, "meter");
    }

    #[test]
    fn sin_zero_dimensionless() {
        let q = Quantity::new(0.0, Unit::dimensionless());
        let r = MathFn::Sin.apply(&[q]).unwrap();
        assert!(r.value.abs() < 1e-12);
        assert!(r.unit.dimensions.is_empty());
    }

    #[test]
    fn sin_dimensioned_fails() {
        let q = Quantity::new(5.0, Unit::meter());
        let err = MathFn::Sin.apply(&[q]).unwrap_err();
        match err {
            RUnitsError::FunctionDomainError { name, .. } => assert_eq!(name, "sin"),
            other => panic!("expected FunctionDomainError, got {other:?}"),
        }
    }

    #[test]
    fn cos_zero_is_one() {
        let q = Quantity::new(0.0, Unit::dimensionless());
        let r = MathFn::Cos.apply(&[q]).unwrap();
        assert!((r.value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn ln_of_e_is_one() {
        let q = Quantity::new(std::f64::consts::E, Unit::dimensionless());
        let r = MathFn::Ln.apply(&[q]).unwrap();
        assert!((r.value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn exp_zero_is_one() {
        let q = Quantity::new(0.0, Unit::dimensionless());
        let r = MathFn::Exp.apply(&[q]).unwrap();
        assert!((r.value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn arity_mismatch_fails() {
        // MathFn::Sqrt is unary; passing 2 args must fail with ArityMismatch.
        let a = Quantity::new(1.0, Unit::dimensionless());
        let b = Quantity::new(2.0, Unit::dimensionless());
        let err = MathFn::Sqrt.apply(&[a, b]).unwrap_err();
        match err {
            RUnitsError::ArityMismatch {
                name,
                expected,
                got,
            } => {
                assert_eq!(name, "sqrt");
                assert_eq!(expected, 1);
                assert_eq!(got, 2);
            }
            other => panic!("expected ArityMismatch, got {other:?}"),
        }
    }

    #[test]
    fn affine_in_function_fails() {
        let q = Quantity::new(20.0, Unit::celsius());
        let err = MathFn::Sqrt.apply(&[q]).unwrap_err();
        assert!(matches!(err, RUnitsError::AffineInExpression { .. }));
    }
}
