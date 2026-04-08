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

mod seed;

use crate::units::Unit;
use crate::units::dimension::Dimension;
use crate::units::unit::ConversionKind;
use std::collections::HashMap;
use std::sync::OnceLock;

/// SI metric prefixes: (long_name, symbol, scale_factor).
/// Sorted by symbol length descending so "da" is tried before "d".
/// SI metric prefixes: (long_name, symbol, scale_factor).
pub const SI_PREFIXES: &[(&str, &str, f64)] = &[
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
        seed::seed_all(&mut units);
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
            // Try long name first, then symbol.
            for prefix in [long, short] {
                if let Some(remainder) = name.strip_prefix(prefix) {
                    if remainder.is_empty() {
                        continue;
                    }
                    if let Some(base_unit) = self.units.get(remainder) {
                        if !base_unit.prefixable {
                            continue;
                        }
                        if info_only && !base_unit.dimensions.contains_key(&Dimension::Information)
                        {
                            continue;
                        }
                        // Build the prefixed unit: canonical name = long prefix + base name
                        // e.g., "kN" → "kilonewton", "µs" → "microsecond"
                        let mut prefixed = base_unit.clone();
                        prefixed.name = format!("{}{}", long, base_unit.name);
                        prefixed.prefixable = false;
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
        assert_eq!(u.dimension_string(), "Length*Time^-1");
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
        assert_eq!(u.dimension_string(), "Length");
    }

    #[test]
    fn si_prefix_symbol() {
        let db = UnitDatabase::new();
        // "Ms" → mega + second = 1e6 factor
        let u = db.lookup("Ms").unwrap();
        assert_eq!(u.conversion_factor(), 1e6);
        assert_eq!(u.dimension_string(), "Time");
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
        assert_eq!(u.dimension_string(), "Information");
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
