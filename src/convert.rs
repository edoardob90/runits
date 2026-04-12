//! Core conversion logic, decoupled from CLI and presentation.
//!
//! This module extracts the parse→convert pipeline into a reusable function
//! that the one-shot CLI, REPL, and batch mode all share.

use crate::annotations::quantity_name;
use crate::database::UnitDatabase;
use crate::error::RUnitsError;
use crate::parser;
use crate::units::Quantity;

/// The structured output of a single conversion, before any formatting.
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// The *evaluated* source quantity (value + source unit). Preserved for
    /// `--explain`. After the Phase 5a expression foundation, this is the
    /// post-eval quantity — for `5 m + 3 ft`, `source` is `5.9144 meter`.
    pub source: Quantity,
    /// The converted quantity (value + target unit).
    pub result: Quantity,
    /// Named physical quantity, if the target dimensions match a known one
    /// (e.g., "Velocity", "Force"). `None` for base dimensions like Length.
    pub annotation: Option<&'static str>,
    /// The original source input string. `--explain` uses this to show the
    /// user's raw expression when it's non-trivial (e.g. `5 m + 3 ft`),
    /// without the noise of echoing it for simple `10 ft` conversions.
    pub source_expr: Option<String>,
}

/// Run a single conversion: parse source and target, convert, annotate.
pub fn run_conversion(
    source: &str,
    target: &str,
    db: &UnitDatabase,
) -> Result<ConversionResult, RUnitsError> {
    let source_qty = parser::parse_quantity(source, db)?;
    let target_unit = parser::parse_unit_name(target, db)?;
    let result = source_qty.clone().convert_to(&target_unit)?;
    let annotation = quantity_name(&result.unit.dimensions);

    Ok(ConversionResult {
        source: source_qty,
        result,
        annotation,
        source_expr: Some(source.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_conversion() {
        let db = UnitDatabase::new();
        let r = run_conversion("10 ft", "m", &db).unwrap();
        assert!((r.result.value - 3.048).abs() < 1e-9);
        assert_eq!(r.annotation, Some("Length"));
    }

    #[test]
    fn annotated_conversion() {
        let db = UnitDatabase::new();
        let r = run_conversion("100 km/h", "m/s", &db).unwrap();
        assert_eq!(r.annotation, Some("Velocity"));
    }

    #[test]
    fn unknown_unit_error() {
        let db = UnitDatabase::new();
        // With expression-based parsing, unknown identifiers land here.
        let err = run_conversion("10 foozle", "m", &db).unwrap_err();
        assert!(matches!(err, RUnitsError::UnknownIdentifier { .. }));
    }

    #[test]
    fn expression_source_succeeds() {
        let db = UnitDatabase::new();
        let r = run_conversion("5 m + 3 ft", "cm", &db).unwrap();
        // source_qty is in meters (LHS wins); converted to cm
        assert_eq!(r.source.unit.name, "meter");
        assert_eq!(r.result.unit.name, "centimeter");
        assert!((r.result.value - 591.44).abs() < 1e-6);
    }
}
