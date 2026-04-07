//! Pest-backed parser with compound-unit expression support (Phase 3).
//!
//! Two public entry points:
//!
//! - [`parse_quantity`] turns `"10 kg*m/s^2"` into a [`Quantity`] by extracting
//!   the number and recursively resolving the unit expression tree.
//! - [`parse_unit_name`] takes a bare unit expression like `"m/s"` or `"kg*m/s^2"`
//!   and resolves it to a composed [`Unit`].
//!
//! Grammar lives in [`grammar.pest`](../grammar.pest). The expression grammar
//! handles `*`, `/`, `^`, parentheses, and implicit multiplication (juxtaposition).

use crate::database::UnitDatabase;
use crate::error::RUnitsError;
use crate::units::{Quantity, Unit};
use pest::Parser;
use pest::iterators::Pair;

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct QuantityParser;

/// Parse a full quantity string (e.g. `"10 ft"`, `"5 kg*m/s^2"`).
///
/// Returns [`RUnitsError::UnknownUnit`] if a unit name isn't in the
/// database, or [`RUnitsError::Parse`] if the input doesn't match the
/// grammar.
pub fn parse_quantity(input: &str, db: &UnitDatabase) -> Result<Quantity, RUnitsError> {
    let mut pairs = QuantityParser::parse(Rule::quantity, input.trim()).map_err(Box::new)?;
    let quantity_pair = pairs.next().expect("grammar guarantees one quantity");
    let mut inner = quantity_pair.into_inner();

    let first = inner.next().expect("grammar guarantees at least unit_expr");
    let (value, unit) = match first.as_rule() {
        Rule::number => {
            let v: f64 = first
                .as_str()
                .parse()
                .expect("grammar validated number format");
            let unit_pair = inner
                .next()
                .expect("grammar guarantees unit_expr after number");
            (v, resolve_unit_expr(unit_pair, db)?)
        }
        Rule::unit_expr => (1.0, resolve_unit_expr(first, db)?),
        other => unreachable!("unexpected rule inside quantity: {:?}", other),
    };

    Ok(Quantity::new(value, unit))
}

/// Parse a bare unit expression (e.g. `"m/s"`, `"kg*m/s^2"`).
///
/// Returns [`RUnitsError::UnknownUnit`] if a name isn't in the database
/// or [`RUnitsError::Parse`] if the input doesn't match the grammar.
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
/// - `unit_expr`: fold terms with division
/// - `term`: fold factors with multiplication
/// - `factor`: resolve atom, then apply exponentiation
/// - `atom`: DB lookup for unit_name, or recurse into parenthesized expr
fn resolve_unit_expr(pair: Pair<Rule>, db: &UnitDatabase) -> Result<Unit, RUnitsError> {
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
        Rule::term => {
            let mut inner = pair.into_inner();
            let mut result = resolve_unit_expr(inner.next().unwrap(), db)?;
            for factor in inner {
                let rhs = resolve_unit_expr(factor, db)?;
                check_affine_composition(&result, &rhs)?;
                result = result * rhs;
            }
            Ok(result)
        }
        Rule::factor => {
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
                Ok(pow_unit(base, exp))
            } else {
                Ok(base)
            }
        }
        Rule::atom => {
            let inner = pair.into_inner().next().unwrap();
            resolve_unit_expr(inner, db)
        }
        Rule::unit_name => {
            let name = pair.as_str();
            db.lookup(name)
                .ok_or_else(|| RUnitsError::UnknownUnit(name.to_string()))
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

/// Raise a unit to an integer power.
///
/// Positive exponents use repeated multiplication; negative exponents
/// invert first (dimensionless / unit), then multiply.
fn pow_unit(unit: Unit, exp: i32) -> Unit {
    if exp == 0 {
        return Unit::dimensionless();
    }
    if exp == 1 {
        return unit;
    }
    if exp < 0 {
        let inv = Unit::dimensionless() / unit;
        return pow_unit(inv, -exp);
    }
    let mut result = unit.clone();
    for _ in 1..exp {
        result = result * unit.clone();
    }
    result
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
        assert_eq!(q.unit.dimension_string(), "length/time");
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
        assert_eq!(q.unit.dimension_string(), "length/time");
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
        assert!(matches!(err, RUnitsError::UnknownUnit(_)));
        assert!(err.to_string().contains("foozle"));
    }

    #[test]
    fn rejects_bad_syntax() {
        let db = UnitDatabase::new();
        let err = parse_quantity("meter 10", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::Parse(_)));
    }

    #[test]
    fn rejects_missing_unit() {
        let db = UnitDatabase::new();
        let err = parse_quantity("10", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::Parse(_)));
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

    // ---- Phase 3 compound expression tests ----

    #[test]
    fn compound_multiplication() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("kg*m", &db).unwrap();
        assert!(u.dimension_string().contains("mass"));
        assert!(u.dimension_string().contains("length"));
    }

    #[test]
    fn compound_with_exponent() {
        let db = UnitDatabase::new();
        // kg*m/s^2 = force dimensions
        let u = parse_unit_name("kg*m/s^2", &db).unwrap();
        let dims = u.dimension_string();
        assert!(dims.contains("mass"));
        assert!(dims.contains("length"));
        assert!(dims.contains("time^2"));
    }

    #[test]
    fn compound_with_spaces() {
        let db = UnitDatabase::new();
        // Spaces around operators should work
        let u = parse_unit_name("kg * m / s ^ 2", &db).unwrap();
        let dims = u.dimension_string();
        assert!(dims.contains("mass"));
        assert!(dims.contains("length"));
        assert!(dims.contains("time^2"));
    }

    #[test]
    fn compound_negative_exponent() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("m^-1", &db).unwrap();
        assert_eq!(u.dimension_string(), "1/length");
    }

    #[test]
    fn compound_parentheses() {
        let db = UnitDatabase::new();
        let u = parse_unit_name("(kg*m)/s^2", &db).unwrap();
        let dims = u.dimension_string();
        assert!(dims.contains("mass"));
        assert!(dims.contains("length"));
        assert!(dims.contains("time^2"));
    }

    #[test]
    fn compound_density() {
        let db = UnitDatabase::new();
        let q = parse_quantity("5 kg/m^3", &db).unwrap();
        assert_eq!(q.value, 5.0);
        let dims = q.unit.dimension_string();
        assert!(dims.contains("mass"));
        assert!(dims.contains("length^3"));
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
}
