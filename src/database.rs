//! In-memory registry of known units + aliases.
//!
//! Phase 2 ships a hand-seeded database of ~80 common units, each with one
//! or more alias strings (`meter`, `m`, `meters`, `metres` all resolve to
//! the same canonical [`Unit`]). The database also holds a handful of
//! pre-built compound aliases (`m/s`, `km/h`, `mph`, `rpm`) so users get
//! velocity / frequency conversions working without Phase 3's compound
//! grammar yet in place.
//!
//! Phase 3 will grow this (SI prefix parsing, GNU `units.dat` ingestion) but
//! the public API — [`UnitDatabase::lookup`] and [`global`] — is expected to
//! stay stable.

use crate::units::Unit;
use crate::units::dimension::Dimension;
use std::collections::HashMap;
use std::sync::OnceLock;

/// A collection of units keyed by every acceptable input alias.
///
/// Every alias gets its own entry pointing to a clone of the canonical
/// [`Unit`]. Lookup is O(1) via `HashMap`, and the returned `Unit` carries
/// the canonical name — so `lookup("ft")` yields a unit whose `name` is
/// `"foot"`. That keeps output predictable regardless of what the user typed.
pub struct UnitDatabase {
    units: HashMap<String, Unit>,
}

impl UnitDatabase {
    /// Build a fresh database with all Phase 2 units + aliases seeded.
    pub fn new() -> Self {
        let mut units = HashMap::new();
        seed_all(&mut units);
        UnitDatabase { units }
    }

    /// Look up a unit by name or alias.
    ///
    /// Returns an owned [`Unit`] (cheap clone: a name `String` + small
    /// `HashMap` of dimensions) or `None` if the alias is unknown. Owned is
    /// simpler than borrowed here — the parser wraps the result in a
    /// `Quantity`, which needs to own its unit anyway.
    pub fn lookup(&self, name: &str) -> Option<Unit> {
        self.units.get(name).cloned()
    }

    /// How many aliases are currently registered (every alias counts).
    pub fn len(&self) -> usize {
        self.units.len()
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }
}

impl Default for UnitDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the process-wide singleton database, initializing on first call.
///
/// Uses [`OnceLock`] (stdlib since Rust 1.70) so we avoid depending on
/// `lazy_static` / `once_cell`. Thread-safe by construction.
pub fn global() -> &'static UnitDatabase {
    static DB: OnceLock<UnitDatabase> = OnceLock::new();
    DB.get_or_init(UnitDatabase::new)
}

/// Helper: register every `alias` as a lookup key for `unit`.
fn add(map: &mut HashMap<String, Unit>, aliases: &[&str], unit: Unit) {
    for alias in aliases {
        map.insert((*alias).to_string(), unit.clone());
    }
}

/// Seeds the ~80 Phase 2 entries. Grouped by dimension for readability.
fn seed_all(map: &mut HashMap<String, Unit>) {
    // ---- SI base units + base extensions ----
    add(map, &["meter", "m", "meters", "metres"], Unit::meter());
    add(map, &["kilogram", "kg", "kilograms"], Unit::kilogram());
    add(map, &["second", "s", "sec", "seconds"], Unit::second());
    add(map, &["ampere", "A", "amps", "amperes"], Unit::ampere());
    add(map, &["kelvin", "K"], Unit::kelvin());
    add(map, &["mole", "mol", "moles"], Unit::mole());
    add(map, &["candela", "cd"], Unit::candela());
    add(map, &["radian", "rad", "radians"], Unit::radian());
    add(map, &["bit", "b", "bits"], Unit::bit());

    // ---- Length ----
    add(
        map,
        &["kilometer", "km", "kilometers", "kilometres"],
        Unit::kilometer(),
    );
    add(
        map,
        &["centimeter", "cm", "centimeters", "centimetres"],
        Unit::new("centimeter", 0.01, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["millimeter", "mm", "millimeters", "millimetres"],
        Unit::new("millimeter", 0.001, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["micrometer", "µm", "um", "micrometers", "micrometres"],
        Unit::new("micrometer", 1e-6, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["nanometer", "nm", "nanometers", "nanometres"],
        Unit::new("nanometer", 1e-9, &[(Dimension::Length, 1)]),
    );
    add(map, &["mile", "mi", "miles"], Unit::mile());
    add(map, &["foot", "ft", "feet"], Unit::foot());
    add(map, &["inch", "in", "inches"], Unit::inch());
    add(
        map,
        &["yard", "yd", "yards"],
        Unit::new("yard", 0.9144, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["nautical_mile", "nmi"],
        Unit::new("nautical_mile", 1852.0, &[(Dimension::Length, 1)]),
    );

    // ---- Time ----
    add(map, &["minute", "min", "minutes"], Unit::minute());
    add(map, &["hour", "hr", "h", "hours"], Unit::hour());
    add(
        map,
        &["day", "d", "days"],
        Unit::new("day", 86_400.0, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["week", "wk", "weeks"],
        Unit::new("week", 604_800.0, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["year", "yr", "years"],
        // Julian year (365.25 days); good enough for Phase 2 scope.
        Unit::new("year", 31_557_600.0, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["millisecond", "ms", "milliseconds"],
        Unit::new("millisecond", 1e-3, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["microsecond", "µs", "us", "microseconds"],
        Unit::new("microsecond", 1e-6, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["nanosecond", "ns", "nanoseconds"],
        Unit::new("nanosecond", 1e-9, &[(Dimension::Time, 1)]),
    );

    // ---- Mass ----
    add(
        map,
        &["gram", "g", "grams"],
        Unit::new("gram", 1e-3, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["milligram", "mg", "milligrams"],
        Unit::new("milligram", 1e-6, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["tonne", "t", "tonnes", "metric_ton"],
        Unit::new("tonne", 1000.0, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["pound", "lb", "lbs", "pounds"],
        Unit::new("pound", 0.45359237, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["ounce", "oz", "ounces"],
        Unit::new("ounce", 0.028349523125, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["stone"],
        Unit::new("stone", 6.35029318, &[(Dimension::Mass, 1)]),
    );

    // ---- Volume (Length^3) ----
    // 1 liter = 1e-3 m^3
    add(
        map,
        &["liter", "l", "L", "litre", "liters", "litres"],
        Unit::new("liter", 1e-3, &[(Dimension::Length, 3)]),
    );
    add(
        map,
        &["milliliter", "ml", "mL", "millilitre"],
        Unit::new("milliliter", 1e-6, &[(Dimension::Length, 3)]),
    );
    // US gallon = 3.785411784 liters = 3.785411784e-3 m^3
    add(
        map,
        &["gallon", "gal", "gallons"],
        Unit::new("gallon", 3.785411784e-3, &[(Dimension::Length, 3)]),
    );
    // US cup = 236.5882365 mL
    add(
        map,
        &["cup", "cups"],
        Unit::new("cup", 2.365882365e-4, &[(Dimension::Length, 3)]),
    );
    // US fluid ounce
    add(
        map,
        &["fluid_ounce", "fl_oz"],
        Unit::new("fluid_ounce", 2.95735295625e-5, &[(Dimension::Length, 3)]),
    );

    // ---- Angles ----
    add(map, &["degree", "deg", "degrees"], Unit::degree());
    add(
        map,
        &["arcminute", "arcmin"],
        // 1 arcmin = (1/60) degree = (π / 10800) rad
        Unit::new(
            "arcminute",
            std::f64::consts::PI / 10_800.0,
            &[(Dimension::Angle, 1)],
        ),
    );
    add(
        map,
        &["arcsecond", "arcsec"],
        Unit::new(
            "arcsecond",
            std::f64::consts::PI / 648_000.0,
            &[(Dimension::Angle, 1)],
        ),
    );

    // ---- Information ----
    add(map, &["byte", "B", "bytes"], Unit::byte());
    add(
        map,
        &["kilobyte", "KB", "kB"],
        // Decimal kilobyte: 1000 bytes = 8000 bits
        Unit::new("kilobyte", 8_000.0, &[(Dimension::Information, 1)]),
    );
    add(
        map,
        &["megabyte", "MB"],
        Unit::new("megabyte", 8_000_000.0, &[(Dimension::Information, 1)]),
    );
    add(
        map,
        &["gigabyte", "GB"],
        Unit::new("gigabyte", 8_000_000_000.0, &[(Dimension::Information, 1)]),
    );
    add(
        map,
        &["terabyte", "TB"],
        Unit::new(
            "terabyte",
            8_000_000_000_000.0,
            &[(Dimension::Information, 1)],
        ),
    );

    // ---- Frequency (Time^-1) ----
    // 1 Hz = 1 / second
    add(
        map,
        &["hertz", "Hz"],
        Unit::new("hertz", 1.0, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["kilohertz", "kHz"],
        Unit::new("kilohertz", 1e3, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["megahertz", "MHz"],
        Unit::new("megahertz", 1e6, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["gigahertz", "GHz"],
        Unit::new("gigahertz", 1e9, &[(Dimension::Time, -1)]),
    );

    // ---- Compound aliases (pre-built via Unit arithmetic) ----
    // These give users velocity / frequency conversions despite Phase 2's
    // grammar not yet supporting compound parsing. Phase 3 will obsolete the
    // "pre-built" trick in favor of dynamic parsing — but the aliases can
    // stay as handy shortcuts.
    let m_per_s = Unit::meter() / Unit::second();
    add(map, &["m/s", "mps"], rename(m_per_s, "meter/second"));

    let km_per_h = Unit::kilometer() / Unit::hour();
    add(
        map,
        &["km/h", "kph", "kmh"],
        rename(km_per_h, "kilometer/hour"),
    );

    let mph = Unit::mile() / Unit::hour();
    add(map, &["mph", "mi/h"], rename(mph, "mile/hour"));

    // rpm: 1 revolution per minute = (1/60) Hz. A revolution is treated as
    // dimensionless (like radians — technically an angle, but idiomatically
    // folded into frequency here).
    add(
        map,
        &["rpm"],
        Unit::new("rpm", 1.0 / 60.0, &[(Dimension::Time, -1)]),
    );
}

/// Rename a unit in-place (the `Mul`/`Div` impls auto-generate names like
/// `"meter/second"` — which is already what we want, but the helper makes
/// the intent explicit at the call site).
fn rename(mut unit: Unit, new_name: &str) -> Unit {
    unit.name = new_name.to_string();
    unit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_canonical_name() {
        let db = UnitDatabase::new();
        let u = db.lookup("meter").unwrap();
        assert_eq!(u.name, "meter");
    }

    #[test]
    fn lookup_short_alias_returns_canonical() {
        let db = UnitDatabase::new();
        let u = db.lookup("ft").unwrap();
        // Input alias is "ft" but the returned Unit's name is canonical.
        assert_eq!(u.name, "foot");
    }

    #[test]
    fn lookup_unknown_returns_none() {
        let db = UnitDatabase::new();
        assert!(db.lookup("foozle").is_none());
    }

    #[test]
    fn compound_alias_m_per_s_is_velocity() {
        let db = UnitDatabase::new();
        let u = db.lookup("m/s").unwrap();
        assert_eq!(u.dimension_string(), "length/time");
    }

    #[test]
    fn km_h_to_mph_conversion() {
        use crate::units::Quantity;
        let db = UnitDatabase::new();
        let kmh = db.lookup("km/h").unwrap();
        let mph = db.lookup("mph").unwrap();
        // 100 km/h ≈ 62.137 mph
        let q = Quantity::new(100.0, kmh);
        let converted = q.convert_to(&mph).unwrap();
        assert!((converted.value - 62.137).abs() < 0.01);
    }

    #[test]
    fn rpm_to_hertz() {
        use crate::units::Quantity;
        let db = UnitDatabase::new();
        let rpm = db.lookup("rpm").unwrap();
        let hz = db.lookup("Hz").unwrap();
        // 60 rpm = 1 Hz
        let q = Quantity::new(60.0, rpm);
        let converted = q.convert_to(&hz).unwrap();
        assert!((converted.value - 1.0).abs() < 1e-9);
    }

    #[test]
    fn global_singleton_is_populated() {
        let db = global();
        assert!(!db.is_empty());
        assert!(db.lookup("m").is_some());
    }
}
