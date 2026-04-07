//! Dimension-signature → physical-quantity name registry.
//!
//! Maps a unit's [`DimensionMap`] to a human-readable quantity name like
//! "Velocity" or "Force". Used to annotate conversion output:
//! `27.7778 meter/second [Velocity]`.
//!
//! The registry covers the ~25 most useful named quantities, using Numbat's
//! `core/dimensions.nbt` as reference.

use crate::units::dimension::{Dimension, DimensionMap};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Look up the physical quantity name for a given dimension signature.
///
/// Returns `None` if the dimensions don't match any known named quantity.
pub fn quantity_name(dims: &DimensionMap) -> Option<&'static str> {
    registry().get(&dim_key(dims)).copied()
}

/// Canonical string key for a DimensionMap: sorted dimension abbreviations
/// with exponents, e.g. "L1M1T-2" for force (kg*m/s^2).
fn dim_key(dims: &DimensionMap) -> String {
    let mut parts: Vec<_> = dims
        .iter()
        .map(|(d, &e)| {
            let c = match d {
                Dimension::Length => "L",
                Dimension::Mass => "M",
                Dimension::Time => "T",
                Dimension::Temperature => "Θ",
                Dimension::Current => "I",
                Dimension::AmountOfSubstance => "N",
                Dimension::LuminousIntensity => "J",
                Dimension::Angle => "A",
                Dimension::Information => "B",
                Dimension::Currency => "$",
            };
            (c, e)
        })
        .collect();
    parts.sort_by_key(|(c, _)| *c);
    parts
        .iter()
        .map(|(c, e)| format!("{}{}", c, e))
        .collect::<Vec<_>>()
        .join("")
}

type Registry = HashMap<String, &'static str>;

fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(build_registry)
}

/// Convenience: build a dim_key from a slice of (Dimension, exponent) pairs.
fn key(pairs: &[(Dimension, i8)]) -> String {
    let map: DimensionMap = pairs.iter().cloned().collect();
    dim_key(&map)
}

fn build_registry() -> Registry {
    let mut r = Registry::new();

    // Mechanical
    r.insert(
        key(&[(Dimension::Length, 1), (Dimension::Time, -1)]),
        "Velocity",
    );
    r.insert(
        key(&[(Dimension::Length, 1), (Dimension::Time, -2)]),
        "Acceleration",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 1),
            (Dimension::Time, -2),
        ]),
        "Force",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, -1),
            (Dimension::Time, -2),
        ]),
        "Pressure",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -2),
        ]),
        "Energy",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -3),
        ]),
        "Power",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 1),
            (Dimension::Time, -1),
        ]),
        "Momentum",
    );

    // Geometric
    r.insert(key(&[(Dimension::Length, 2)]), "Area");
    r.insert(key(&[(Dimension::Length, 3)]), "Volume");
    r.insert(
        key(&[(Dimension::Mass, 1), (Dimension::Length, -3)]),
        "Density",
    );

    // Temporal
    r.insert(key(&[(Dimension::Time, -1)]), "Frequency");

    // Electromagnetic
    r.insert(
        key(&[(Dimension::Current, 1), (Dimension::Time, 1)]),
        "Electric Charge",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -3),
            (Dimension::Current, -1),
        ]),
        "Voltage",
    );
    r.insert(
        key(&[
            (Dimension::Mass, -1),
            (Dimension::Length, -2),
            (Dimension::Time, 4),
            (Dimension::Current, 2),
        ]),
        "Capacitance",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -3),
            (Dimension::Current, -2),
        ]),
        "Electric Resistance",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -2),
            (Dimension::Current, -1),
        ]),
        "Magnetic Flux",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -2),
            (Dimension::Current, -2),
        ]),
        "Inductance",
    );
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Time, -2),
            (Dimension::Current, -1),
        ]),
        "Magnetic Flux Density",
    );

    // Thermodynamic
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 2),
            (Dimension::Time, -2),
            (Dimension::Temperature, -1),
        ]),
        "Entropy",
    );

    // Photometric
    r.insert(
        key(&[(Dimension::LuminousIntensity, 1), (Dimension::Angle, 2)]),
        "Luminous Flux",
    );

    // Radiation
    r.insert(
        key(&[(Dimension::Length, 2), (Dimension::Time, -2)]),
        "Absorbed Dose",
    );

    // Angular
    r.insert(
        key(&[(Dimension::Angle, 1), (Dimension::Time, -1)]),
        "Angular Velocity",
    );

    // Viscosity
    r.insert(
        key(&[
            (Dimension::Mass, 1),
            (Dimension::Length, -1),
            (Dimension::Time, -1),
        ]),
        "Dynamic Viscosity",
    );
    r.insert(
        key(&[(Dimension::Length, 2), (Dimension::Time, -1)]),
        "Kinematic Viscosity",
    );

    // Data
    r.insert(
        key(&[(Dimension::Information, 1), (Dimension::Time, -1)]),
        "Data Rate",
    );

    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims(pairs: &[(Dimension, i8)]) -> DimensionMap {
        pairs.iter().cloned().collect()
    }

    #[test]
    fn velocity_annotation() {
        let d = dims(&[(Dimension::Length, 1), (Dimension::Time, -1)]);
        assert_eq!(quantity_name(&d), Some("Velocity"));
    }

    #[test]
    fn force_annotation() {
        let d = dims(&[
            (Dimension::Mass, 1),
            (Dimension::Length, 1),
            (Dimension::Time, -2),
        ]);
        assert_eq!(quantity_name(&d), Some("Force"));
    }

    #[test]
    fn unknown_dimensions_return_none() {
        let d = dims(&[(Dimension::Length, 1)]);
        assert_eq!(quantity_name(&d), None);
    }

    #[test]
    fn frequency_annotation() {
        let d = dims(&[(Dimension::Time, -1)]);
        assert_eq!(quantity_name(&d), Some("Frequency"));
    }
}
