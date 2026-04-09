//! Dimension-signature → physical-quantity name registry.
//!
//! Maps a unit's [`DimensionMap`] to a human-readable quantity name like
//! "Velocity" or "Force". Used to annotate conversion output:
//! `27.7778 meter/second` with annotation "Velocity".
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

/// Number of named quantities in the registry.
pub fn quantity_name_count() -> usize {
    registry().len()
}

/// Reverse lookup: find the dimension signature for a named quantity.
///
/// Case-insensitive. Returns `None` if the name doesn't match any known quantity.
/// ```
/// use runits::annotations::dimensions_for_name;
/// use runits::units::dimension::Dimension;
///
/// let dims = dimensions_for_name("velocity").unwrap();
/// assert_eq!(dims.get(&Dimension::Length), Some(&1));
/// assert_eq!(dims.get(&Dimension::Time), Some(&-1));
/// ```
pub fn dimensions_for_name(name: &str) -> Option<DimensionMap> {
    let name_lower = name.to_lowercase();
    for (dim_key_str, &qty_name) in registry() {
        if qty_name.to_lowercase() == name_lower {
            return Some(parse_dim_key(dim_key_str));
        }
    }
    None
}

/// Return all registered quantity names, sorted alphabetically.
pub fn all_quantity_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = registry().values().copied().collect();
    names.sort();
    names.dedup();
    names
}

/// Parse a canonical dim_key string back into a DimensionMap.
///
/// Input format: "L1M1T-2" → {Length: 1, Mass: 1, Time: -2}
fn parse_dim_key(key: &str) -> DimensionMap {
    let mut map = DimensionMap::new();
    let mut chars = key.chars().peekable();

    while let Some(c) = chars.next() {
        let dim = match c {
            'L' => Dimension::Length,
            'M' => Dimension::Mass,
            'T' => Dimension::Time,
            'I' => Dimension::Current,
            'J' => Dimension::LuminousIntensity,
            'N' => Dimension::AmountOfSubstance,
            'A' => Dimension::Angle,
            'B' => Dimension::Information,
            '$' => Dimension::Currency,
            'Θ' => Dimension::Temperature,
            _ => continue,
        };

        // Parse the exponent digits (including leading minus).
        let mut exp_str = String::new();
        while let Some(&next) = chars.peek() {
            if next == '-' || next.is_ascii_digit() {
                exp_str.push(next);
                chars.next();
            } else {
                break;
            }
        }
        if let Ok(exp) = exp_str.parse::<i8>() {
            map.insert(dim, exp);
        }
    }
    map
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

    // Base dimensions (single-dimension entries for bare quantity echo)
    r.insert(key(&[(Dimension::Length, 1)]), "Length");
    r.insert(key(&[(Dimension::Mass, 1)]), "Mass");
    r.insert(key(&[(Dimension::Time, 1)]), "Time");
    r.insert(key(&[(Dimension::Temperature, 1)]), "Temperature");
    r.insert(key(&[(Dimension::Current, 1)]), "Current");
    r.insert(key(&[(Dimension::AmountOfSubstance, 1)]), "Amount");
    r.insert(
        key(&[(Dimension::LuminousIntensity, 1)]),
        "Luminous Intensity",
    );
    r.insert(key(&[(Dimension::Angle, 1)]), "Angle");
    r.insert(key(&[(Dimension::Information, 1)]), "Information");

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
    fn base_dimension_annotates() {
        let d = dims(&[(Dimension::Length, 1)]);
        assert_eq!(quantity_name(&d), Some("Length"));
    }

    #[test]
    fn unknown_dimensions_return_none() {
        // A combination not in the registry.
        let d = dims(&[(Dimension::Length, 4)]);
        assert_eq!(quantity_name(&d), None);
    }

    #[test]
    fn frequency_annotation() {
        let d = dims(&[(Dimension::Time, -1)]);
        assert_eq!(quantity_name(&d), Some("Frequency"));
    }

    // ---- Reverse lookup tests ----

    #[test]
    fn dimensions_for_velocity() {
        let d = dimensions_for_name("velocity").unwrap();
        assert_eq!(d.get(&Dimension::Length), Some(&1));
        assert_eq!(d.get(&Dimension::Time), Some(&-1));
    }

    #[test]
    fn dimensions_for_name_case_insensitive() {
        assert!(dimensions_for_name("Velocity").is_some());
        assert!(dimensions_for_name("VELOCITY").is_some());
        assert!(dimensions_for_name("velocity").is_some());
    }

    #[test]
    fn dimensions_for_unknown_returns_none() {
        assert!(dimensions_for_name("nonexistent").is_none());
    }

    #[test]
    fn all_quantity_names_contains_expected() {
        let names = all_quantity_names();
        assert!(names.contains(&"Velocity"));
        assert!(names.contains(&"Force"));
        assert!(names.contains(&"Length"));
        assert!(names.contains(&"Energy"));
    }

    #[test]
    fn all_quantity_names_sorted_and_deduped() {
        let names = all_quantity_names();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(names, sorted);
    }

    #[test]
    fn roundtrip_dim_key_parse() {
        // Verify that dimensions_for_name gives back what quantity_name expects.
        let d = dimensions_for_name("force").unwrap();
        assert_eq!(quantity_name(&d), Some("Force"));
    }
}
