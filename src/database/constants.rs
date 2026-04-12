//! Physical constants database.
//!
//! Parallel to the unit registry: a `ConstantsDatabase` holding named physical
//! constants (speed of light, Planck constant, etc.) with alias lookup.
//! Constants are semantically distinct from units — each is a `Quantity`
//! (a specific numeric value with dimensions), not a conversion scale.
//!
//! Naming follows Numbat's `physics/constants.nbt` conventions where possible.
//! Aliases are chosen to avoid collisions with unit names and SI prefix symbols
//! (e.g., `c` is the centi prefix, so speed of light is `speed_of_light`/`c_0`).

use crate::units::Unit;
use crate::units::dimension::Dimension;
use std::collections::HashMap;
use std::sync::OnceLock;

/// A named physical constant with a numeric value and unit.
#[derive(Debug, Clone)]
pub struct Constant {
    /// Canonical name (descriptive, e.g. "speed_of_light").
    pub name: &'static str,
    /// Numeric value in the given unit.
    pub value: f64,
    /// The unit this value is expressed in (e.g., m/s for speed of light).
    pub unit: Unit,
    /// Short human-readable description.
    pub description: &'static str,
}

/// A collection of physical constants keyed by every acceptable alias.
///
/// Follows the same pattern as [`super::UnitDatabase`]: every alias gets its
/// own entry pointing to a clone of the canonical `Constant`.
pub struct ConstantsDatabase {
    constants: HashMap<String, Constant>,
}

impl ConstantsDatabase {
    /// Build a fresh database with all builtin constants seeded.
    pub fn new() -> Self {
        let mut constants = HashMap::new();
        seed_all(&mut constants);
        ConstantsDatabase { constants }
    }

    /// Look up a constant by name or alias.
    pub fn lookup(&self, name: &str) -> Option<&Constant> {
        self.constants.get(name)
    }

    /// Iterate over all registered alias strings.
    pub fn constant_names(&self) -> impl Iterator<Item = &str> {
        self.constants.keys().map(|s| s.as_str())
    }

    /// Suggest the closest known constant names for a misspelled input.
    ///
    /// Mirrors `UnitDatabase::suggest`: Jaro-Winkler with a 0.7 cutoff,
    /// deduplicated by canonical constant name. Used by the expression
    /// evaluator's `UnknownIdentifier` error path.
    pub fn suggest(&self, unknown: &str, max: usize) -> Vec<String> {
        use std::collections::HashSet;

        let unknown_lower = unknown.to_lowercase();
        let mut scored: Vec<_> = self
            .constants
            .iter()
            .map(|(alias, constant)| {
                let score = strsim::jaro_winkler(&unknown_lower, &alias.to_lowercase());
                (alias.as_str(), constant.name, score)
            })
            .filter(|(_, _, score)| *score > 0.7)
            .collect();
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        let mut seen = HashSet::new();
        scored
            .into_iter()
            .filter(|(_, canonical, _)| seen.insert(*canonical))
            .take(max)
            .map(|(alias, _, _)| alias.to_string())
            .collect()
    }

    /// Return all unique constants (deduplicated by canonical name).
    pub fn all_unique(&self) -> Vec<&Constant> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        self.constants
            .values()
            .filter(|c| seen.insert(c.name))
            .collect()
    }

    /// How many aliases are registered (every alias counts).
    pub fn len(&self) -> usize {
        self.constants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }
}

impl Default for ConstantsDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the process-wide singleton constants database.
pub fn global() -> &'static ConstantsDatabase {
    static DB: OnceLock<ConstantsDatabase> = OnceLock::new();
    DB.get_or_init(ConstantsDatabase::new)
}

/// Helper: register every `alias` as a lookup key for `constant`.
fn add(map: &mut HashMap<String, Constant>, aliases: &[&str], constant: Constant) {
    for alias in aliases {
        map.insert((*alias).to_string(), constant.clone());
    }
}

/// Seeds the builtin physical constants.
///
/// Values are CODATA 2018 recommended values unless noted.
/// Naming follows Numbat's `physics/constants.nbt` conventions.
fn seed_all(map: &mut HashMap<String, Constant>) {
    // ---- Fundamental constants ----

    add(
        map,
        &["speed_of_light", "c_0"],
        Constant {
            name: "speed_of_light",
            value: 299_792_458.0,
            unit: Unit::meter() / Unit::second(),
            description: "Speed of light in vacuum",
        },
    );

    add(
        map,
        &["gravitational_constant"],
        Constant {
            name: "gravitational_constant",
            value: 6.674_30e-11,
            unit: Unit::new(
                "m³/(kg·s²)",
                1.0,
                &[
                    (Dimension::Length, 3),
                    (Dimension::Mass, -1),
                    (Dimension::Time, -2),
                ],
            ),
            description: "Newtonian constant of gravitation",
        },
    );

    add(
        map,
        &["gravity", "g0", "g_n"],
        Constant {
            name: "gravity",
            value: 9.806_65,
            unit: Unit::new(
                "m/s²",
                1.0,
                &[(Dimension::Length, 1), (Dimension::Time, -2)],
            ),
            description: "Standard acceleration of gravity on Earth",
        },
    );

    add(
        map,
        &["planck_constant"],
        Constant {
            name: "planck_constant",
            value: 6.626_070_15e-34,
            unit: Unit::new(
                "J·s",
                1.0,
                &[
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -1),
                ],
            ),
            description: "Planck constant",
        },
    );

    add(
        map,
        &["hbar", "h_bar", "ℏ"],
        Constant {
            name: "hbar",
            value: 1.054_571_817e-34,
            unit: Unit::new(
                "J·s",
                1.0,
                &[
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -1),
                ],
            ),
            description: "Reduced Planck constant (h/2π)",
        },
    );

    // ---- Thermodynamic / statistical ----

    add(
        map,
        &["boltzmann_constant", "k_B"],
        Constant {
            name: "boltzmann_constant",
            value: 1.380_649e-23,
            unit: Unit::new(
                "J/K",
                1.0,
                &[
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -2),
                    (Dimension::Temperature, -1),
                ],
            ),
            description: "Boltzmann constant",
        },
    );

    add(
        map,
        &["avogadro_constant", "N_A"],
        Constant {
            name: "avogadro_constant",
            value: 6.022_140_76e23,
            unit: Unit::new("mol⁻¹", 1.0, &[(Dimension::AmountOfSubstance, -1)]),
            description: "Avogadro constant",
        },
    );

    add(
        map,
        &["gas_constant", "R_gas"],
        Constant {
            name: "gas_constant",
            value: 8.314_462_618,
            unit: Unit::new(
                "J/(mol·K)",
                1.0,
                &[
                    (Dimension::Mass, 1),
                    (Dimension::Length, 2),
                    (Dimension::Time, -2),
                    (Dimension::AmountOfSubstance, -1),
                    (Dimension::Temperature, -1),
                ],
            ),
            description: "Molar gas constant",
        },
    );

    // ---- Electromagnetic ----

    add(
        map,
        &["elementary_charge", "electron_charge"],
        Constant {
            name: "elementary_charge",
            value: 1.602_176_634e-19,
            unit: Unit::new("C", 1.0, &[(Dimension::Current, 1), (Dimension::Time, 1)]),
            description: "Elementary charge",
        },
    );

    add(
        map,
        &["electric_constant", "eps0", "ε0"],
        Constant {
            name: "electric_constant",
            value: 8.854_187_812_8e-12,
            unit: Unit::new(
                "F/m",
                1.0,
                &[
                    (Dimension::Current, 2),
                    (Dimension::Time, 4),
                    (Dimension::Mass, -1),
                    (Dimension::Length, -3),
                ],
            ),
            description: "Vacuum electric permittivity",
        },
    );

    add(
        map,
        &["magnetic_constant", "mu0", "µ0"],
        Constant {
            name: "magnetic_constant",
            value: 1.256_637_062_12e-6,
            unit: Unit::new(
                "N/A²",
                1.0,
                &[
                    (Dimension::Mass, 1),
                    (Dimension::Length, 1),
                    (Dimension::Time, -2),
                    (Dimension::Current, -2),
                ],
            ),
            description: "Vacuum magnetic permeability",
        },
    );

    // ---- Particle masses ----

    add(
        map,
        &["electron_mass"],
        Constant {
            name: "electron_mass",
            value: 9.109_383_701_5e-31,
            unit: Unit::kilogram(),
            description: "Electron mass",
        },
    );

    add(
        map,
        &["proton_mass"],
        Constant {
            name: "proton_mass",
            value: 1.672_621_923_69e-27,
            unit: Unit::kilogram(),
            description: "Proton mass",
        },
    );

    // ---- Dimensionless / atomic ----

    add(
        map,
        &["fine_structure_constant", "alpha", "α"],
        Constant {
            name: "fine_structure_constant",
            value: 7.297_352_569_3e-3,
            unit: Unit::dimensionless(),
            description: "Fine-structure constant",
        },
    );

    add(
        map,
        &["bohr_radius", "a0"],
        Constant {
            name: "bohr_radius",
            value: 5.291_772_109_03e-11,
            unit: Unit::meter(),
            description: "Bohr radius",
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::UnitDatabase;

    #[test]
    fn lookup_by_canonical_name() {
        let db = ConstantsDatabase::new();
        let c = db.lookup("speed_of_light").unwrap();
        assert_eq!(c.name, "speed_of_light");
        assert!((c.value - 299_792_458.0).abs() < 1.0);
    }

    #[test]
    fn lookup_by_alias() {
        let db = ConstantsDatabase::new();
        // c_0 is an alias for speed_of_light
        let c = db.lookup("c_0").unwrap();
        assert_eq!(c.name, "speed_of_light");

        // k_B is an alias for boltzmann_constant
        let c = db.lookup("k_B").unwrap();
        assert_eq!(c.name, "boltzmann_constant");

        // g0 is an alias for gravity
        let c = db.lookup("g0").unwrap();
        assert_eq!(c.name, "gravity");
    }

    #[test]
    fn all_constants_have_correct_dimensions() {
        let db = ConstantsDatabase::new();
        // speed_of_light: L*T^-1
        let c = db.lookup("speed_of_light").unwrap();
        assert_eq!(c.unit.dimension_string(), "Length*Time^-1");

        // gravity: L*T^-2
        let g = db.lookup("gravity").unwrap();
        assert_eq!(g.unit.dimension_string(), "Length*Time^-2");

        // boltzmann_constant: L^2*M*T^-2*Θ^-1
        let kb = db.lookup("k_B").unwrap();
        assert!(kb.unit.dimensions.contains_key(&Dimension::Temperature));

        // fine_structure_constant: dimensionless
        let alpha = db.lookup("alpha").unwrap();
        assert!(alpha.unit.dimensions.is_empty());

        // electron_mass: M
        let me = db.lookup("electron_mass").unwrap();
        assert_eq!(me.unit.dimension_string(), "Mass");
    }

    #[test]
    fn no_alias_collides_with_unit_db() {
        let const_db = ConstantsDatabase::new();
        let unit_db = UnitDatabase::new();

        for alias in const_db.constant_names() {
            assert!(
                unit_db.lookup(alias).is_none(),
                "constant alias '{alias}' collides with a unit in UnitDatabase"
            );
        }
    }

    #[test]
    fn all_unique_returns_15_constants() {
        let db = ConstantsDatabase::new();
        let unique = db.all_unique();
        assert_eq!(unique.len(), 15);
    }

    #[test]
    fn global_singleton_is_populated() {
        let db = global();
        assert!(!db.is_empty());
        assert!(db.lookup("speed_of_light").is_some());
    }

    #[test]
    fn unicode_aliases_work() {
        let db = ConstantsDatabase::new();
        assert!(db.lookup("ℏ").is_some());
        assert!(db.lookup("ε0").is_some());
        assert!(db.lookup("µ0").is_some());
        assert!(db.lookup("α").is_some());
    }
}
