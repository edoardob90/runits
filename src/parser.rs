//! Pest-backed parser for Phase 2 CLI input.
//!
//! Two public entry points:
//!
//! - [`parse_quantity`] turns `"10 ft"` into a [`Quantity`] by extracting the
//!   number, looking up the unit name in the supplied [`UnitDatabase`], and
//!   packaging both together.
//! - [`parse_unit_name`] takes a bare unit reference like `"m"` or `"km/h"`
//!   and resolves it to a [`Unit`]. Used for the CLI's target argument.
//!
//! Grammar lives in [`grammar.pest`](../grammar.pest). Phase 3 will replace
//! `unit_name` with a proper compound-unit expression tree; until then,
//! compound names like `"m/s"` are matched as single tokens against the
//! database's pre-built aliases.

use crate::database::UnitDatabase;
use crate::error::RUnitsError;
use crate::units::{Quantity, Unit};
use pest::Parser;

#[derive(pest_derive::Parser)]
#[grammar = "grammar.pest"]
pub struct QuantityParser;

/// Parse a full quantity string (e.g. `"10 ft"`, `"6.022e23 mole"`).
///
/// Returns [`RUnitsError::UnknownUnit`] if the unit name isn't in the
/// database, or [`RUnitsError::Parse`] if the input doesn't match the
/// grammar.
pub fn parse_quantity(input: &str, db: &UnitDatabase) -> Result<Quantity, RUnitsError> {
    let mut pairs = QuantityParser::parse(Rule::quantity, input.trim()).map_err(Box::new)?;

    // The outer `quantity` pair contains either (number, unit_name) when a
    // value was given, or just (unit_name) for a bare unit query like
    // "mile" (shorthand for "1 mile"). WHITESPACE is silent so it never
    // shows up in the pair tree.
    let quantity_pair = pairs.next().expect("grammar guarantees one quantity");
    let mut inner = quantity_pair.into_inner();

    let first = inner.next().expect("grammar guarantees at least unit_name");
    let (value, unit_str) = match first.as_rule() {
        Rule::number => {
            // `number` rule already validated the format, so parse() can't fail
            // for structural reasons. The .expect is reachable only on f64
            // overflow, which we accept as a panic for Phase 2.
            let v: f64 = first
                .as_str()
                .parse()
                .expect("grammar validated number format");
            let u = inner
                .next()
                .expect("grammar guarantees unit_name after number")
                .as_str();
            (v, u)
        }
        Rule::unit_name => (1.0, first.as_str()),
        other => unreachable!("unexpected rule inside quantity: {:?}", other),
    };

    let unit = db
        .lookup(unit_str)
        .ok_or_else(|| RUnitsError::UnknownUnit(unit_str.to_string()))?;

    Ok(Quantity::new(value, unit))
}

/// Parse a bare unit reference (e.g. `"m"`, `"km/h"`).
///
/// Returns [`RUnitsError::UnknownUnit`] if the name isn't in the database
/// or [`RUnitsError::Parse`] if the input contains characters the grammar
/// rejects (whitespace, digits-first, etc.).
pub fn parse_unit_name(input: &str, db: &UnitDatabase) -> Result<Unit, RUnitsError> {
    let mut pairs = QuantityParser::parse(Rule::unit_only, input.trim()).map_err(Box::new)?;

    let unit_only_pair = pairs.next().expect("grammar guarantees one unit_only");
    let mut inner = unit_only_pair.into_inner();
    let unit_str = inner.next().expect("grammar guarantees unit_name").as_str();

    db.lookup(unit_str)
        .ok_or_else(|| RUnitsError::UnknownUnit(unit_str.to_string()))
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
    fn parses_compound_alias() {
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
        // "mile" alone is shorthand for "1 mile" — the canonical
        // "how many km in a mile?" question.
        let db = UnitDatabase::new();
        let q = parse_quantity("mile", &db).unwrap();
        assert_eq!(q.value, 1.0);
        assert_eq!(q.unit.name, "mile");
    }

    #[test]
    fn parses_bare_compound_alias() {
        let db = UnitDatabase::new();
        let q = parse_quantity("km/h", &db).unwrap();
        assert_eq!(q.value, 1.0);
        assert_eq!(q.unit.name, "kilometer/hour");
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
    fn parse_unit_name_rejects_whitespace() {
        let db = UnitDatabase::new();
        // An inner space should fail the grammar — no leading/trailing
        // whitespace issue because we trim().
        let err = parse_unit_name("kilo gram", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::Parse(_)));
    }
}
