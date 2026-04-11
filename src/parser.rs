//! Pest-backed parser with full source-side expression support.
//!
//! Two public entry points:
//!
//! - [`parse_quantity`] turns a user input like `"10 kg*m/s^2"`, `"5 m + 3 ft"`,
//!   or `"sqrt(9 m^2)"` into a [`Quantity`] by parsing it into an [`Expr`]
//!   AST and evaluating under a default [`EvalContext`]. This is a thin
//!   wrapper around [`parse_and_eval`].
//! - [`parse_unit_name`] takes a bare unit expression like `"m/s"` or
//!   `"kg*m/s^2"` and resolves it to a composed [`Unit`]. Target-side
//!   parsing stays pure — no math, no identifiers beyond unit names.
//!
//! Grammar lives in [`grammar.pest`](../grammar.pest). Expression grammar
//! is documented there; the AST walker lives in [`crate::expr`] and the
//! tree walker / evaluator lives in [`crate::eval`].

use crate::database::UnitDatabase;
use crate::error::RUnitsError;
use crate::eval::{EvalContext, eval};
use crate::expr::parse_expression;
use crate::units::{Quantity, Unit};
use pest::Parser;
use pest::iterators::Pair;

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct QuantityParser;

/// Parse a full quantity string and evaluate it under a default context.
///
/// Equivalent to `parse_and_eval(input, &EvalContext::one_shot(db))`. Used by
/// the CLI one-shot and batch paths, which don't have a REPL `previous`.
pub fn parse_quantity(input: &str, db: &UnitDatabase) -> Result<Quantity, RUnitsError> {
    parse_and_eval(input, &EvalContext::one_shot(db))
}

/// Parse an expression and evaluate it under a caller-provided context.
///
/// The REPL uses this to supply a `previous` quantity for the `_` variable.
pub fn parse_and_eval(input: &str, ctx: &EvalContext) -> Result<Quantity, RUnitsError> {
    let expr = parse_expression(input)?;
    eval(&expr, ctx)
}

/// Parse a bare unit expression (e.g. `"m/s"`, `"kg*m/s^2"`).
///
/// Target-side parser — no math, no identifiers beyond unit names. Returns
/// [`RUnitsError::UnknownUnit`] if a name isn't in the database or
/// [`RUnitsError::Parse`] if the input doesn't match the grammar.
pub fn parse_unit_name(input: &str, db: &UnitDatabase) -> Result<Unit, RUnitsError> {
    let mut pairs = QuantityParser::parse(Rule::unit_only, input.trim()).map_err(Box::new)?;
    let unit_only_pair = pairs.next().expect("grammar guarantees one unit_only");
    let unit_expr_pair = unit_only_pair
        .into_inner()
        .next()
        .expect("grammar guarantees unit_expr");
    resolve_unit_expr(unit_expr_pair, db)
}

/// Recursively resolve a pest `unit_expr` parse tree into a composed [`Unit`].
///
/// Walks the tree according to operator precedence:
/// - `unit_expr`: fold `unit_term`s with division
/// - `unit_term`: fold `unit_factor`s with multiplication
/// - `unit_factor`: resolve `unit_atom`, then apply exponentiation
/// - `unit_atom`: DB lookup for `unit_name`, or recurse into parenthesized expr
pub(crate) fn resolve_unit_expr(pair: Pair<Rule>, db: &UnitDatabase) -> Result<Unit, RUnitsError> {
    match pair.as_rule() {
        Rule::unit_expr => {
            let mut inner = pair.into_inner();
            let mut result = resolve_unit_expr(inner.next().unwrap(), db)?;
            for term in inner {
                let rhs = resolve_unit_expr(term, db)?;
                check_affine_composition(&result, &rhs)?;
                result = result / rhs;
            }
            Ok(result)
        }
        Rule::unit_term => {
            let mut inner = pair.into_inner();
            let mut result = resolve_unit_expr(inner.next().unwrap(), db)?;
            for factor in inner {
                let rhs = resolve_unit_expr(factor, db)?;
                check_affine_composition(&result, &rhs)?;
                result = result * rhs;
            }
            Ok(result)
        }
        Rule::unit_factor => {
            let mut inner = pair.into_inner();
            let base = resolve_unit_expr(inner.next().unwrap(), db)?;
            if let Some(exp_pair) = inner.next() {
                let exp: i32 = exp_pair
                    .as_str()
                    .parse()
                    .expect("grammar validated integer");
                if base.is_affine() {
                    return Err(RUnitsError::AffineComposition(base.name.clone()));
                }
                Ok(crate::units::unit::pow_unit(base, exp))
            } else {
                Ok(base)
            }
        }
        Rule::unit_atom => {
            let inner = pair.into_inner().next().unwrap();
            resolve_unit_expr(inner, db)
        }
        Rule::unit_name => {
            let name = pair.as_str();
            db.lookup(name).ok_or_else(|| RUnitsError::UnknownUnit {
                suggestions: db.suggest(name, 3),
                name: name.to_string(),
            })
        }
        _ => unreachable!("unexpected rule in unit expression: {:?}", pair.as_rule()),
    }
}

/// Check that neither operand is affine before composing.
fn check_affine_composition(lhs: &Unit, rhs: &Unit) -> Result<(), RUnitsError> {
    if lhs.is_affine() {
        return Err(RUnitsError::AffineComposition(lhs.name.clone()));
    }
    if rhs.is_affine() {
        return Err(RUnitsError::AffineComposition(rhs.name.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_quantity() {
        let db = UnitDatabase::new();
        let q = parse_quantity("10 meter", &db).unwrap();
        assert_eq!(q.value, 10.0);
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn parses_short_alias() {
        let db = UnitDatabase::new();
        let q = parse_quantity("5 ft", &db).unwrap();
        assert_eq!(q.value, 5.0);
        assert_eq!(q.unit.name, "foot");
    }

    #[test]
    fn parses_decimal_value() {
        let db = UnitDatabase::new();
        let q = parse_quantity("2.5 foot", &db).unwrap();
        assert!((q.value - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_scientific_notation() {
        let db = UnitDatabase::new();
        let q = parse_quantity("6.022e23 mole", &db).unwrap();
        assert!((q.value - 6.022e23).abs() < 1e15);
    }

    #[test]
    fn parses_negative_number() {
        let db = UnitDatabase::new();
        let q = parse_quantity("-2.5 meter", &db).unwrap();
        assert!((q.value + 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn parses_negative_exponent() {
        let db = UnitDatabase::new();
        let q = parse_quantity("9.81E-2 meter", &db).unwrap();
        assert!((q.value - 0.0981).abs() < 1e-6);
    }

    #[test]
    fn parses_compound_division() {
        let db = UnitDatabase::new();
        let q = parse_quantity("100 km/h", &db).unwrap();
        assert_eq!(q.value, 100.0);
        assert_eq!(q.unit.dimension_string(), "Length*Time^-1");
    }

    #[test]
    fn parses_micro_prefix_alias() {
        let db = UnitDatabase::new();
        let q = parse_quantity("50 µs", &db).unwrap();
        assert_eq!(q.value, 50.0);
        assert_eq!(q.unit.name, "microsecond");
    }

    #[test]
    fn parses_bare_unit_name_as_value_one() {
        let db = UnitDatabase::new();
        let q = parse_quantity("mile", &db).unwrap();
        assert_eq!(q.value, 1.0);
        assert_eq!(q.unit.name, "mile");
    }

    #[test]
    fn parses_bare_compound_expr() {
        let db = UnitDatabase::new();
        let q = parse_quantity("km/h", &db).unwrap();
        assert_eq!(q.value, 1.0);
        assert_eq!(q.unit.dimension_string(), "Length*Time^-1");
    }

    #[test]
    fn bare_short_alias_resolves_to_canonical() {
        let db = UnitDatabase::new();
        let q = parse_quantity("ft", &db).unwrap();
        assert_eq!(q.value, 1.0);
        assert_eq!(q.unit.name, "foot");
    }

    #[test]
    fn rejects_unknown_unit() {
        let db = UnitDatabase::new();
        let err = parse_quantity("10 foozle", &db).unwrap_err();
        // Source-side parser uses UnknownIdentifier (looks up units AND
        // constants); the distinction is intentional — see error.rs docs.
        assert!(matches!(err, RUnitsError::UnknownIdentifier { .. }));
        assert!(err.to_string().contains("foozle"));
    }

    #[test]
    fn rejects_bad_syntax() {
        let db = UnitDatabase::new();
        // `meter 10` parses as `meter * 10` (juxtaposition) now, so the
        // previous "meter 10 is invalid" expectation no longer holds. Use a
        // clearly malformed input instead.
        let err = parse_quantity("10 @ meter", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::Parse(_)));
    }

    #[test]
    fn parses_bare_number() {
        let db = UnitDatabase::new();
        // With full expression support, `10` is a valid dimensionless scalar.
        // The old "bare number" rejection no longer makes sense at the parser
        // level — it fails later at `convert_to` for dimension mismatch.
        let q = parse_quantity("10", &db).unwrap();
        assert_eq!(q.value, 10.0);
        assert!(q.unit.dimensions.is_empty());
    }

    #[test]
    fn parse_unit_name_resolves_alias() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("kg", &db).unwrap();
        assert_eq!(u.name, "kilogram");
    }

    #[test]
    fn parse_unit_name_rejects_whitespace_only_content() {
        let db = UnitDatabase::new();
        let err = parse_unit_name("   ", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::Parse(_)));
    }

    // ---- Compound expression tests ----

    #[test]
    fn compound_multiplication() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("kg*m", &db).unwrap();
        assert!(u.dimension_string().contains("Mass"));
        assert!(u.dimension_string().contains("Length"));
    }

    #[test]
    fn compound_with_exponent() {
        let db = UnitDatabase::new();
        // kg*m/s^2 = force dimensions
        let u = parse_unit_name("kg*m/s^2", &db).unwrap();
        let dims = u.dimension_string();
        assert!(dims.contains("Mass"));
        assert!(dims.contains("Length"));
        assert!(dims.contains("Time^-2"));
    }

    #[test]
    fn compound_with_spaces() {
        let db = UnitDatabase::new();
        // Spaces around operators should work
        let u = parse_unit_name("kg * m / s ^ 2", &db).unwrap();
        let dims = u.dimension_string();
        assert!(dims.contains("Mass"));
        assert!(dims.contains("Length"));
        assert!(dims.contains("Time^-2"));
    }

    #[test]
    fn compound_negative_exponent() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("m^-1", &db).unwrap();
        assert_eq!(u.dimension_string(), "Length^-1");
    }

    #[test]
    fn compound_parentheses() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("(kg*m)/s^2", &db).unwrap();
        let dims = u.dimension_string();
        assert!(dims.contains("Mass"));
        assert!(dims.contains("Length"));
        assert!(dims.contains("Time^-2"));
    }

    #[test]
    fn compound_density() {
        let db = UnitDatabase::new();
        let q = parse_quantity("5 kg/m^3", &db).unwrap();
        assert_eq!(q.value, 5.0);
        let dims = q.unit.dimension_string();
        assert!(dims.contains("Mass"));
        assert!(dims.contains("Length^-3"));
    }

    #[test]
    fn affine_unit_in_compound_rejected() {
        let db = UnitDatabase::new();
        let err = parse_unit_name("celsius*m", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::AffineComposition(_)));
    }

    #[test]
    fn dimensionless_from_zero_exponent() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("m^0", &db).unwrap();
        assert!(u.dimensions.is_empty());
    }

    // ---- New Phase 5a expression tests ----

    #[test]
    fn parses_negative_scientific() {
        let db = UnitDatabase::new();
        let q = parse_quantity("-2e-3 m", &db).unwrap();
        assert!((q.value + 2e-3).abs() < 1e-12);
        assert_eq!(q.unit.name, "meter");
    }

    #[test]
    fn parses_double_negation() {
        let db = UnitDatabase::new();
        let q = parse_quantity("-(-2 m)", &db).unwrap();
        assert!((q.value - 2.0).abs() < 1e-12);
    }

    #[test]
    fn parses_func_call_in_expression() {
        let db = UnitDatabase::new();
        let q = parse_quantity("sin(0) m", &db).unwrap();
        assert!(q.value.abs() < 1e-12);
    }

    #[test]
    fn underscore_prefix_is_not_a_partial_parse() {
        let db = UnitDatabase::new();
        let err = parse_quantity("_foo m", &db).unwrap_err();
        // Must be a clean parse error, not a silent partial-`_` success.
        assert!(matches!(err, RUnitsError::Parse(_)));
    }
}
