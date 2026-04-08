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
use crate::units::unit::ConversionKind;
use std::collections::HashMap;
use std::sync::OnceLock;

/// SI metric prefixes: (long_name, symbol, scale_factor).
/// Sorted by symbol length descending so "da" is tried before "d".
const SI_PREFIXES: &[(&str, &str, f64)] = &[
    ("yotta", "Y", 1e24),
    ("zetta", "Z", 1e21),
    ("exa", "E", 1e18),
    ("peta", "P", 1e15),
    ("tera", "T", 1e12),
    ("giga", "G", 1e9),
    ("mega", "M", 1e6),
    ("kilo", "k", 1e3),
    ("hecto", "h", 1e2),
    ("deca", "da", 1e1),
    ("deci", "d", 1e-1),
    ("centi", "c", 1e-2),
    ("milli", "m", 1e-3),
    ("micro", "µ", 1e-6),
    ("nano", "n", 1e-9),
    ("pico", "p", 1e-12),
    ("femto", "f", 1e-15),
    ("atto", "a", 1e-18),
    ("zepto", "z", 1e-21),
    ("yocto", "y", 1e-24),
];

/// IEC binary prefixes for information units only.
const BINARY_PREFIXES: &[(&str, &str, f64)] = &[
    ("kibi", "Ki", 1024.0),
    ("mebi", "Mi", 1_048_576.0),
    ("gibi", "Gi", 1_073_741_824.0),
    ("tebi", "Ti", 1_099_511_627_776.0),
    ("pebi", "Pi", 1_125_899_906_842_624.0),
    ("exbi", "Ei", 1_152_921_504_606_846_976.0),
];

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
    /// First tries a direct lookup (O(1)). If that fails, tries stripping
    /// SI prefixes (e.g., "Gmeter" → giga + meter) and binary prefixes
    /// (e.g., "kibibyte" → kibi + byte). Direct lookup always wins, so
    /// existing aliases like "min" (minute) are never misinterpreted as
    /// "m" (milli) + "in" (inch).
    pub fn lookup(&self, name: &str) -> Option<Unit> {
        // 1. Direct lookup — fast path, handles all seeded aliases.
        if let Some(u) = self.units.get(name) {
            return Some(u.clone());
        }
        // 2. Try SI prefix stripping (any dimension).
        if let Some(u) = self.try_prefix_strip(name, SI_PREFIXES, false) {
            return Some(u);
        }
        // 3. Try binary prefix stripping (information units only).
        if let Some(u) = self.try_prefix_strip(name, BINARY_PREFIXES, true) {
            return Some(u);
        }
        // FUTURE(alias-types): No case-insensitive fallback here. Unit symbols
        // are case-sensitive (Mm ≠ mm, K ≠ k, Ci ≠ ci). When the database
        // gains symbol vs full-name alias distinction (see Numbat's short/both/
        // none modes), enable case-insensitive lookup for full names only.
        None
    }

    /// Try to split `name` into a known prefix + a known base unit.
    ///
    /// For each prefix, tries the long form first ("kilo"), then the symbol
    /// ("k"). Within symbols, longer prefixes are tried first (the table is
    /// pre-sorted) so "da" beats "d".
    ///
    /// When `info_only` is true, only matches base units with an Information
    /// dimension (prevents "kibibar" or similar nonsense).
    fn try_prefix_strip(
        &self,
        name: &str,
        prefixes: &[(&str, &str, f64)],
        info_only: bool,
    ) -> Option<Unit> {
        for &(long, short, scale) in prefixes {
            // Try long name first, then symbol — both in the same loop.
            for prefix in [long, short] {
                if let Some(remainder) = name.strip_prefix(prefix) {
                    if remainder.is_empty() {
                        continue;
                    }
                    if let Some(base_unit) = self.units.get(remainder) {
                        // Skip affine units — "kilocelsius" is nonsense.
                        if base_unit.is_affine() {
                            continue;
                        }
                        // When restricted to info, check the base unit's dimensions.
                        if info_only && !base_unit.dimensions.contains_key(&Dimension::Information)
                        {
                            continue;
                        }
                        // Build the prefixed unit with scaled factor.
                        let mut prefixed = base_unit.clone();
                        prefixed.name = name.to_string();
                        match &mut prefixed.conversion {
                            ConversionKind::Linear(f) => *f *= scale,
                            ConversionKind::Affine { .. } => continue,
                        }
                        return Some(prefixed);
                    }
                }
            }
        }
        None
    }

    /// Iterate over all registered alias strings.
    ///
    /// Exposes the HashMap keys without exposing `Unit` values. Used by
    /// fuzzy matching and REPL tab-completion.
    pub fn unit_names(&self) -> impl Iterator<Item = &str> {
        self.units.keys().map(|s| s.as_str())
    }

    /// Suggest the closest known unit names for a misspelled input.
    ///
    /// FUTURE(alias-types): Scoring is case-insensitive, which is safe for
    /// suggestions (advisory, not resolution). When symbol vs name aliases
    /// are distinguished, suggestions could rank name-matches higher.
    ///
    /// Uses Jaro-Winkler similarity (weights prefix matches heavily — good
    /// for typos where the first few characters are correct). Returns up to
    /// `max` suggestions, deduplicated by canonical unit name.
    pub fn suggest(&self, unknown: &str, max: usize) -> Vec<String> {
        use std::collections::HashSet;

        // Compare lowercased for case-insensitive scoring, return original alias.
        let unknown_lower = unknown.to_lowercase();
        let mut scored: Vec<_> = self
            .units
            .iter()
            .map(|(alias, unit)| {
                let score = strsim::jaro_winkler(&unknown_lower, &alias.to_lowercase());
                (alias.as_str(), unit.name.as_str(), score)
            })
            .filter(|(_, _, score)| *score > 0.7)
            .collect();
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        // Deduplicate by canonical name (many aliases → same unit).
        let mut seen = HashSet::new();
        scored
            .into_iter()
            .filter(|(_, canonical, _)| seen.insert(canonical.to_string()))
            .take(max)
            .map(|(alias, _, _)| alias.to_string())
            .collect()
    }

    /// Find all canonical unit names compatible with `unit` (same dimensions).
    ///
    /// Returns deduplicated canonical names, sorted alphabetically,
    /// excluding the query unit's own canonical name.
    pub fn compatible_units(&self, unit: &Unit) -> Vec<String> {
        use std::collections::BTreeSet;
        let mut names = BTreeSet::new();
        for u in self.units.values() {
            if u.is_compatible_with(unit) && u.name != unit.name {
                names.insert(u.name.clone());
            }
        }
        names.into_iter().collect()
    }

    /// Find all alias strings that map to a given canonical unit name.
    ///
    /// Returns sorted aliases, excluding the canonical name itself.
    pub fn aliases_for(&self, canonical: &str) -> Vec<String> {
        let mut aliases: Vec<String> = self
            .units
            .iter()
            .filter(|(alias, unit)| unit.name == canonical && alias.as_str() != canonical)
            .map(|(alias, _)| alias.clone())
            .collect();
        aliases.sort();
        aliases
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
    // Absolute temperature scales (affine conversions)
    add(map, &["celsius", "degC", "°C"], Unit::celsius());
    add(map, &["fahrenheit", "degF", "°F"], Unit::fahrenheit());
    add(map, &["rankine", "Ra"], Unit::rankine());
    add(map, &["reaumur", "Re", "°Re"], Unit::reaumur());
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

    // ---- Force (M·L·T⁻²) ----
    let force_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, 1),
        (Dimension::Time, -2),
    ];
    add(
        map,
        &["newton", "N", "newtons"],
        Unit::new("newton", 1.0, force_dims),
    );
    add(map, &["dyne", "dyn"], Unit::new("dyne", 1e-5, force_dims));
    add(
        map,
        &["pound_force", "lbf"],
        Unit::new("pound_force", 4.448222, force_dims),
    );
    add(
        map,
        &["kilogram_force", "kgf"],
        Unit::new("kilogram_force", 9.80665, force_dims),
    );

    // ---- Pressure (M·L⁻¹·T⁻²) ----
    let pressure_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, -1),
        (Dimension::Time, -2),
    ];
    add(
        map,
        &["pascal", "Pa", "pascals"],
        Unit::new("pascal", 1.0, pressure_dims),
    );
    add(map, &["bar", "bars"], Unit::new("bar", 1e5, pressure_dims));
    add(
        map,
        &["atmosphere", "atm"],
        Unit::new("atmosphere", 101_325.0, pressure_dims),
    );
    add(
        map,
        &["torr", "Torr"],
        Unit::new("torr", 101_325.0 / 760.0, pressure_dims),
    );
    add(
        map,
        &["mmHg"],
        Unit::new("mmHg", 133.322387415, pressure_dims),
    );
    add(
        map,
        &["psi", "PSI"],
        Unit::new("psi", 6894.757, pressure_dims),
    );
    add(map, &["inHg"], Unit::new("inHg", 3386.389, pressure_dims));

    // ---- Energy (M·L²·T⁻²) ----
    let energy_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, 2),
        (Dimension::Time, -2),
    ];
    add(
        map,
        &["joule", "J", "joules"],
        Unit::new("joule", 1.0, energy_dims),
    );
    add(
        map,
        &["calorie", "cal", "calories"],
        Unit::new("calorie", 4.184, energy_dims),
    );
    add(
        map,
        &["BTU", "Btu"],
        Unit::new("BTU", 1055.05585262, energy_dims),
    );
    add(
        map,
        &["kilowatt_hour", "kWh"],
        Unit::new("kilowatt_hour", 3.6e6, energy_dims),
    );
    add(
        map,
        &["electronvolt", "eV"],
        Unit::new("electronvolt", 1.602_176_634e-19, energy_dims),
    );

    // ---- Power (M·L²·T⁻³) ----
    let power_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, 2),
        (Dimension::Time, -3),
    ];
    add(
        map,
        &["watt", "W", "watts"],
        Unit::new("watt", 1.0, power_dims),
    );
    add(
        map,
        &["horsepower", "hp"],
        Unit::new("horsepower", 735.49875, power_dims),
    );

    // ---- Historical length ----
    add(
        map,
        &["furlong", "furlongs"],
        Unit::new("furlong", 201.168, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["league", "leagues"],
        Unit::new("league", 4828.032, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["fathom", "fathoms"],
        Unit::new("fathom", 1.8288, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["rod", "rods"],
        Unit::new("rod", 5.0292, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["chain", "chains"],
        Unit::new("chain", 20.1168, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["cubit", "cubits"],
        Unit::new("cubit", 0.4572, &[(Dimension::Length, 1)]),
    );

    // ---- Cooking (volume) ----
    // Volume dims: L³
    let volume_dims = &[(Dimension::Length, 3)];
    add(
        map,
        &["tablespoon", "tbsp"],
        Unit::new("tablespoon", 1.5e-5, volume_dims),
    );
    add(
        map,
        &["teaspoon", "tsp"],
        Unit::new("teaspoon", 5e-6, volume_dims),
    );

    // ---- Astronomical ----
    add(
        map,
        &["astronomical_unit", "AU", "au"],
        Unit::new(
            "astronomical_unit",
            1.495_978_707e11,
            &[(Dimension::Length, 1)],
        ),
    );
    add(
        map,
        &["light_year", "ly"],
        Unit::new("light_year", 9.4607e15, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["parsec", "pc"],
        Unit::new("parsec", 3.0857e16, &[(Dimension::Length, 1)]),
    );

    // ---- Radioactivity ----
    add(
        map,
        &["becquerel", "Bq"],
        Unit::new("becquerel", 1.0, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["curie", "Ci"],
        Unit::new("curie", 3.7e10, &[(Dimension::Time, -1)]),
    );
    // Absorbed dose (L²·T⁻²)
    let dose_dims = &[(Dimension::Length, 2), (Dimension::Time, -2)];
    add(map, &["gray", "Gy"], Unit::new("gray", 1.0, dose_dims));
    add(
        map,
        &["sievert", "Sv"],
        Unit::new("sievert", 1.0, dose_dims),
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
        assert_eq!(u.dimension_string(), "length*time^-1");
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

    // ---- SI prefix stripping tests ----

    #[test]
    fn si_prefix_long_name() {
        let db = UnitDatabase::new();
        // "Gmeter" → giga + meter = 1e9 factor
        let u = db.lookup("Gmeter").unwrap();
        assert_eq!(u.conversion_factor(), 1e9);
        assert_eq!(u.dimension_string(), "length");
    }

    #[test]
    fn si_prefix_symbol() {
        let db = UnitDatabase::new();
        // "Ms" → mega + second = 1e6 factor
        let u = db.lookup("Ms").unwrap();
        assert_eq!(u.conversion_factor(), 1e6);
        assert_eq!(u.dimension_string(), "time");
    }

    #[test]
    fn si_prefix_does_not_override_direct_alias() {
        let db = UnitDatabase::new();
        // "min" is minute (direct alias), NOT milli + "in" (inch)
        let u = db.lookup("min").unwrap();
        assert_eq!(u.name, "minute");
    }

    #[test]
    fn si_prefix_existing_prefixed_unit_wins() {
        let db = UnitDatabase::new();
        // "km" is already in DB as "kilometer" — direct lookup wins
        let u = db.lookup("km").unwrap();
        assert_eq!(u.name, "kilometer");
    }

    #[test]
    fn si_prefix_skips_affine_units() {
        let db = UnitDatabase::new();
        // "kilocelsius" should not match — celsius is affine
        assert!(db.lookup("kilocelsius").is_none());
    }

    #[test]
    fn binary_prefix_long_name() {
        let db = UnitDatabase::new();
        // "kibibyte" → kibi + byte = 1024 * 8 bits
        let u = db.lookup("kibibyte").unwrap();
        assert_eq!(u.conversion_factor(), 8.0 * 1024.0);
        assert_eq!(u.dimension_string(), "information");
    }

    #[test]
    fn binary_prefix_symbol() {
        let db = UnitDatabase::new();
        // "Kibyte" → Ki + byte = 1024 * 8 bits
        let u = db.lookup("Kibyte").unwrap();
        assert_eq!(u.conversion_factor(), 8.0 * 1024.0);
    }

    #[test]
    fn binary_prefix_restricted_to_info() {
        let db = UnitDatabase::new();
        // "kibiliter" should NOT match — liter is not an information unit
        assert!(db.lookup("kibiliter").is_none());
    }

    #[test]
    fn si_prefix_deca_before_deci() {
        let db = UnitDatabase::new();
        // "dag" → should try "da" (deca) + "g" (gram) = 10 * 0.001 = 0.01
        // NOT "d" (deci) + "ag" (no such unit)
        let u = db.lookup("dag").unwrap();
        assert!((u.conversion_factor() - 0.01).abs() < 1e-15);
    }

    // ---- Fuzzy suggestion tests ----

    #[test]
    fn suggest_typo_returns_matches() {
        let db = UnitDatabase::new();
        let suggestions = db.suggest("meterr", 3);
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"meter".to_string()));
    }

    #[test]
    fn suggest_gibberish_returns_empty() {
        let db = UnitDatabase::new();
        let suggestions = db.suggest("xyzzy", 3);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn suggest_deduplicates_by_canonical_name() {
        let db = UnitDatabase::new();
        // "meters" and "metres" both map to canonical "meter" — only one should appear.
        let suggestions = db.suggest("meter", 5);
        let meter_count = suggestions.iter().filter(|s| *s == "meter").count();
        assert!(meter_count <= 1);
    }

    #[test]
    fn unit_names_yields_all_aliases() {
        let db = UnitDatabase::new();
        let names: Vec<&str> = db.unit_names().collect();
        assert!(names.contains(&"meter"));
        assert!(names.contains(&"ft"));
        assert!(names.contains(&"degC"));
    }

    // ---- Compatible units + aliases tests ----

    #[test]
    fn compatible_units_for_meter() {
        let db = UnitDatabase::new();
        let meter = db.lookup("meter").unwrap();
        let compat = db.compatible_units(&meter);
        assert!(compat.contains(&"foot".to_string()));
        assert!(compat.contains(&"mile".to_string()));
        assert!(compat.contains(&"inch".to_string()));
        assert!(!compat.contains(&"meter".to_string())); // excludes self
        assert!(!compat.contains(&"second".to_string())); // wrong dimension
    }

    #[test]
    fn compatible_units_for_newton() {
        let db = UnitDatabase::new();
        let newton = db.lookup("N").unwrap();
        let compat = db.compatible_units(&newton);
        assert!(compat.contains(&"dyne".to_string()));
        assert!(compat.contains(&"pound_force".to_string()));
    }

    #[test]
    fn aliases_for_meter() {
        let db = UnitDatabase::new();
        let aliases = db.aliases_for("meter");
        assert!(aliases.contains(&"m".to_string()));
        assert!(aliases.contains(&"meters".to_string()));
        assert!(aliases.contains(&"metres".to_string()));
        assert!(!aliases.contains(&"meter".to_string())); // excludes canonical
    }

    #[test]
    fn aliases_for_unknown_returns_empty() {
        let db = UnitDatabase::new();
        assert!(db.aliases_for("xyzzy").is_empty());
    }
}
