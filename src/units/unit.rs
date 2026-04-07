//! The Unit object.
//!
//! This module defines what a Unit is and how to create common units.
//! A unit is a measure of a physical quantity, such as length, mass, or time.
//! Units are the building blocks of a "System of Units" (e.g., the SI, CGS or Gaussian),
//! and there must be a given number of **base units** from which all other units can be derived.
//! Each unit has a common name (plus an arbitrary number of short-names or symbols),
//! a [`ConversionKind`] (linear factor or affine scale+offset) to convert to/from
//! the base unit, and a full specification of its dimensions (stored as a [`DimensionMap`]).

use super::dimension::{Dimension, DimensionMap, create_dimensions};
use std::ops::{Div, Mul};

/// How a unit converts to/from its dimension's base unit.
///
/// Most units are [`Linear`](ConversionKind::Linear): the value in base units
/// is simply `value * factor` (e.g., 1 foot = 0.3048 meters).
///
/// Temperature scales like Celsius and Fahrenheit are
/// [`Affine`](ConversionKind::Affine): they need both a scale *and* an offset
/// (`to_base = value * scale + offset`). Affine units **cannot** participate
/// in compound-unit multiplication or division — "celsius * meter" is
/// physically meaningless.
///
/// Adding a third variant later (e.g., logarithmic for dB) will trigger
/// exhaustive-match errors at every use site, forcing correct handling.
#[derive(Debug, Clone)]
pub enum ConversionKind {
    /// Pure scaling: `to_base = value * factor`.
    Linear(f64),
    /// Scale + offset: `to_base = value * scale + offset`.
    /// Used for absolute temperature scales (Celsius, Fahrenheit, Réaumur).
    Affine { scale: f64, offset: f64 },
}

/// Represents a unit of measurement in a system of units.
///
/// A Unit is a fundamental building block in a system of units, representing a specific quantity
/// with a name, conversion kind (linear or affine), and dimensions. Units are used to express
/// physical quantities in a consistent and standardized manner.
///
/// # Examples
///
/// ```
/// use runits::units::unit::Unit;
/// use runits::units::dimension::Dimension;
///
/// // Create basic units
/// let meter = Unit::new("meter", 1.0, &[(Dimension::Length, 1)]);
/// let second = Unit::new("second", 1.0, &[(Dimension::Time, 1)]);
///
/// // Check dimensional compatibility first
/// let foot = Unit::new("foot", 0.3048, &[(Dimension::Length, 1)]);
/// assert!(meter.is_compatible_with(&foot));
///
/// // Combine units with arithmetic (consumes the units)
/// let velocity = meter / second;
/// assert_eq!(velocity.name, "meter/second");
/// assert_eq!(velocity.conversion_factor(), 1.0);
/// ```
#[derive(Debug, Clone)]
pub struct Unit {
    /// The unit's base name.
    ///
    /// Strict rules to follow:
    /// 1. No plurals ("meter" and not "meters")
    /// 2. Lowercase ("Newton" is the physicist, while "newton" is the SI unit of the force)
    pub name: String,
    /// How this unit converts to/from the base unit for its dimension.
    pub conversion: ConversionKind,
    /// What this unit measures: `{Length: 1}` for meters, `{Mass: 1, Length: 1, Time: -2}` for newtons
    pub dimensions: DimensionMap,
}

impl Unit {
    /// Creates a new unit with the specified name, conversion factor, and dimensions.
    ///
    /// # Arguments
    /// * `name` - The unit's name (should be lowercase, singular)
    /// * `conversion_factor` - Linear scale factor to the base unit (1.0 for base units)
    /// * `dimensions` - List of (dimension, exponent) pairs defining what this unit measures
    ///
    /// # Examples
    /// ```
    /// use runits::units::unit::Unit;
    /// use runits::units::dimension::Dimension;
    ///
    /// // Base unit
    /// let meter = Unit::new("meter", 1.0, &[(Dimension::Length, 1)]);
    ///
    /// // Derived unit
    /// let foot = Unit::new("foot", 0.3048, &[(Dimension::Length, 1)]);
    ///
    /// // Compound unit
    /// let newton = Unit::new("newton", 1.0, &[
    ///     (Dimension::Mass, 1),
    ///     (Dimension::Length, 1),
    ///     (Dimension::Time, -2)
    /// ]);
    /// ```
    pub fn new(name: &str, conversion_factor: f64, dimensions: &[(Dimension, i8)]) -> Self {
        Unit {
            name: name.to_string(),
            conversion: ConversionKind::Linear(conversion_factor),
            dimensions: create_dimensions(dimensions),
        }
    }

    /// Creates a unit with an affine (scale + offset) conversion to the base unit.
    ///
    /// The conversion formula is: `base_value = value * scale + offset`.
    /// Used for absolute temperature scales.
    pub fn new_affine(name: &str, scale: f64, offset: f64, dimensions: &[(Dimension, i8)]) -> Self {
        Unit {
            name: name.to_string(),
            conversion: ConversionKind::Affine { scale, offset },
            dimensions: create_dimensions(dimensions),
        }
    }

    /// Convert a value in this unit to the dimension's base unit.
    ///
    /// For linear units: `value * factor`.
    /// For affine units: `value * scale + offset`.
    pub fn to_base_value(&self, value: f64) -> f64 {
        match &self.conversion {
            ConversionKind::Linear(factor) => value * factor,
            ConversionKind::Affine { scale, offset } => value * scale + offset,
        }
    }

    /// Convert a value from the dimension's base unit to this unit.
    ///
    /// For linear units: `base_value / factor`.
    /// For affine units: `(base_value - offset) / scale`.
    pub fn from_base_value(&self, base_value: f64) -> f64 {
        match &self.conversion {
            ConversionKind::Linear(factor) => base_value / factor,
            ConversionKind::Affine { scale, offset } => (base_value - offset) / scale,
        }
    }

    /// Returns `true` if this unit uses an affine conversion (has an offset).
    pub fn is_affine(&self) -> bool {
        matches!(&self.conversion, ConversionKind::Affine { .. })
    }

    /// Returns the linear conversion factor.
    ///
    /// # Panics
    /// Panics if the unit is affine. Use [`to_base_value`](Self::to_base_value)
    /// / [`from_base_value`](Self::from_base_value) for general conversions.
    pub fn conversion_factor(&self) -> f64 {
        match &self.conversion {
            ConversionKind::Linear(factor) => *factor,
            ConversionKind::Affine { .. } => {
                panic!("conversion_factor() called on affine unit '{}'", self.name)
            }
        }
    }

    // ===== Factory methods for units =====

    // ----- SI BASE UNITS -----
    // These are the fundamental units with linear factor = 1.0

    // Length (SI: meter)
    pub fn meter() -> Self {
        Self::new("meter", 1.0, &[(Dimension::Length, 1)])
    }

    // Mass (SI: kilogram)
    pub fn kilogram() -> Self {
        Self::new("kilogram", 1.0, &[(Dimension::Mass, 1)])
    }

    // Time (SI: second)
    pub fn second() -> Self {
        Self::new("second", 1.0, &[(Dimension::Time, 1)])
    }

    // Temperature (SI: kelvin)
    pub fn kelvin() -> Self {
        Self::new("kelvin", 1.0, &[(Dimension::Temperature, 1)])
    }

    // Electric current (SI: ampere)
    pub fn ampere() -> Self {
        Self::new("ampere", 1.0, &[(Dimension::Current, 1)])
    }

    // Amount of substance (SI: mole)
    pub fn mole() -> Self {
        Self::new("mole", 1.0, &[(Dimension::AmountOfSubstance, 1)])
    }

    // Luminous intensity (SI: candela)
    pub fn candela() -> Self {
        Self::new("candela", 1.0, &[(Dimension::LuminousIntensity, 1)])
    }

    // ----- SPECIAL UNITS -----

    /// A dimensionless unit with factor 1.0. Used as the identity element
    /// for unit exponentiation (m^0 = dimensionless).
    pub fn dimensionless() -> Self {
        Self::new("dimensionless", 1.0, &[])
    }

    // ----- OTHER BASE UNITS (non-SI) -----

    // Angle (radian)
    pub fn radian() -> Self {
        Self::new("radian", 1.0, &[(Dimension::Angle, 1)])
    }

    // Information (bit)
    pub fn bit() -> Self {
        Self::new("bit", 1.0, &[(Dimension::Information, 1)])
    }

    // ----- TEMPERATURE SCALES (affine) -----
    // Base unit is kelvin (linear, factor 1.0). Other scales use affine
    // conversions: to_base = value * scale + offset.

    // Celsius: to_base(v) = v * 1.0 + 273.15
    pub fn celsius() -> Self {
        Self::new_affine("celsius", 1.0, 273.15, &[(Dimension::Temperature, 1)])
    }

    // Fahrenheit: to_base(v) = v * (5/9) + (459.67 * 5/9)
    // Verification: 32°F → 32 * 5/9 + 255.3722... = 17.7778 + 255.3722 = 273.15 K = 0°C ✓
    pub fn fahrenheit() -> Self {
        Self::new_affine(
            "fahrenheit",
            5.0 / 9.0,
            459.67 * 5.0 / 9.0,
            &[(Dimension::Temperature, 1)],
        )
    }

    // Rankine: absolute scale with Fahrenheit-sized degrees. to_base(v) = v * 5/9
    // Linear — no offset (0 Ra = 0 K).
    pub fn rankine() -> Self {
        Self::new("rankine", 5.0 / 9.0, &[(Dimension::Temperature, 1)])
    }

    // Réaumur: to_base(v) = v * 1.25 + 273.15
    // Verification: 0°Ré → 0 * 1.25 + 273.15 = 273.15 K = 0°C ✓
    //              80°Ré → 80 * 1.25 + 273.15 = 373.15 K = 100°C ✓
    pub fn reaumur() -> Self {
        Self::new_affine("reaumur", 1.25, 273.15, &[(Dimension::Temperature, 1)])
    }

    // ----- DERIVED UNITS -----
    // These units are defined in terms of base units

    // Length derived units
    pub fn kilometer() -> Self {
        Self::new("kilometer", 1000.0, &[(Dimension::Length, 1)])
    }

    pub fn mile() -> Self {
        // 1 mile = 1609.344 meters
        Self::new("mile", 1609.344, &[(Dimension::Length, 1)])
    }

    pub fn foot() -> Self {
        // 1 foot = 0.3048 meters
        Self::new("foot", 0.3048, &[(Dimension::Length, 1)])
    }

    pub fn inch() -> Self {
        // 1 inch = 0.0254 meters
        Self::new("inch", 0.0254, &[(Dimension::Length, 1)])
    }

    // Time derived units
    pub fn minute() -> Self {
        Self::new("minute", 60.0, &[(Dimension::Time, 1)])
    }

    pub fn hour() -> Self {
        Self::new("hour", 3600.0, &[(Dimension::Time, 1)])
    }

    // Angle derived units
    pub fn degree() -> Self {
        Self::new(
            "degree",
            std::f64::consts::PI / 180.0,
            &[(Dimension::Angle, 1)],
        )
    }

    // Information derived units
    pub fn byte() -> Self {
        Self::new("byte", 8.0, &[(Dimension::Information, 1)])
    }

    /// Checks if two units measure the same physical quantity.
    ///
    /// Units are compatible if they have identical dimensions, meaning they can
    /// be converted between each other. For example, meters and feet are both
    /// length units, so they're compatible.
    ///
    /// # Examples
    /// ```
    /// use runits::units::unit::Unit;
    /// use runits::units::dimension::Dimension;
    ///
    /// let meter = Unit::new("meter", 1.0, &[(Dimension::Length, 1)]);
    /// let foot = Unit::new("foot", 0.3048, &[(Dimension::Length, 1)]);
    /// let second = Unit::new("second", 1.0, &[(Dimension::Time, 1)]);
    ///
    /// assert!(meter.is_compatible_with(&foot));  // Both are length
    /// assert!(!meter.is_compatible_with(&second)); // Length ≠ Time
    /// ```
    pub fn is_compatible_with(&self, other: &Unit) -> bool {
        self.dimensions == other.dimensions
    }

    /// Returns a human-readable description of what this unit measures.
    ///
    /// Converts the unit's dimensional formula into a readable string format.
    /// Positive exponents appear in the numerator, negative in the denominator.
    ///
    /// # Examples
    /// ```
    /// use runits::units::unit::Unit;
    /// use runits::units::dimension::Dimension;
    ///
    /// let meter = Unit::new("meter", 1.0, &[(Dimension::Length, 1)]);
    /// assert_eq!(meter.dimension_string(), "length");
    ///
    /// let velocity = Unit::new("velocity", 1.0, &[
    ///     (Dimension::Length, 1),
    ///     (Dimension::Time, -1)
    /// ]);
    /// assert_eq!(velocity.dimension_string(), "length/time");
    ///
    /// let acceleration = Unit::new("acceleration", 1.0, &[
    ///     (Dimension::Length, 1),
    ///     (Dimension::Time, -2)
    /// ]);
    /// assert_eq!(acceleration.dimension_string(), "length/time^2");
    /// ```
    pub fn dimension_string(&self) -> String {
        // Convert {Length: 1, Time: -1} into "length/time"
        // Examples:
        // - {Length: 1} -> "length"
        // - {Length: 1, Time: -1} -> "length/time"
        // - {Mass: 1, Length: 1, Time: -2} -> "mass*length/time^2"
        // - {Length: 2} -> "length^2"
        let mut numerator: Vec<String> = Vec::new();
        let mut denominator: Vec<String> = Vec::new();

        // Loop over the dimensions
        for (dimension, &exponent) in self.dimensions.iter() {
            // We need a String not a &str
            let dimension_name = dimension.name().to_string();
            // Check the exponent
            let dimension_str = if exponent.abs() == 1 {
                dimension_name
            } else {
                format!("{}^{}", dimension_name, exponent.abs())
            };
            // Build the numerator or denominator
            if exponent > 0 {
                numerator.push(dimension_str);
            } else {
                denominator.push(dimension_str);
            }
        }

        // Combine the numerator & denominator with correct separators
        let numerator_str = numerator.join("*");
        let denominator_str = denominator.join("*");

        if denominator_str.is_empty() {
            numerator_str
        } else if numerator_str.is_empty() {
            format!("1/{}", denominator_str)
        } else {
            format!("{}/{}", numerator_str, denominator_str)
        }
    }

    /// Render this unit's dimensions as base SI unit symbols with exponents.
    ///
    /// Unlike [`dimension_string`](Self::dimension_string) which uses dimension
    /// names ("length", "mass"), this uses SI symbols ("m", "kg", "s", etc.).
    /// Used by the `--to-base` CLI flag.
    pub fn to_base_unit_string(&self) -> String {
        let mut numerator: Vec<String> = Vec::new();
        let mut denominator: Vec<String> = Vec::new();

        for (dimension, &exponent) in self.dimensions.iter() {
            let symbol = dimension.base_symbol();
            let part = if exponent.abs() == 1 {
                symbol.to_string()
            } else {
                format!("{}^{}", symbol, exponent.abs())
            };
            if exponent > 0 {
                numerator.push(part);
            } else {
                denominator.push(part);
            }
        }

        let num = numerator.join("*");
        let den = denominator.join("*");

        if den.is_empty() {
            if num.is_empty() {
                "dimensionless".to_string()
            } else {
                num
            }
        } else if num.is_empty() {
            format!("1/{}", den)
        } else {
            format!("{}/{}", num, den)
        }
    }
}

impl PartialEq for Unit {
    // Two units are equal if they have the same name and dimensions
    // We don't compare conversion values in case there are rounding errors
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.dimensions == other.dimensions
    }
}

// Implement multiplication for units: meter * second
// Affine units (temperature) must not be composed — the parser rejects them
// before reaching here, but the debug_assert catches programming errors.
impl Mul for Unit {
    type Output = Unit; // The result of multiplying two Units is a Unit

    fn mul(self, rhs: Unit) -> Unit {
        debug_assert!(
            !self.is_affine() && !rhs.is_affine(),
            "cannot multiply affine units: '{}' * '{}'",
            self.name,
            rhs.name
        );
        let result_unit_name = format!("{}*{}", self.name, rhs.name);
        let mut result_dimensions: DimensionMap = self.dimensions.clone();
        for (dimension, &exponent) in rhs.dimensions.iter() {
            *result_dimensions.entry(dimension.clone()).or_insert(0) += exponent;
        }
        result_dimensions.retain(|_, &mut exp| exp != 0);
        let dimensions_vec: Vec<(Dimension, i8)> = result_dimensions.into_iter().collect();
        Unit::new(
            &result_unit_name,
            self.conversion_factor() * rhs.conversion_factor(),
            &dimensions_vec,
        )
    }
}

// Implement division for units: meter / second = m/s
impl Div for Unit {
    type Output = Unit;

    fn div(self, rhs: Unit) -> Unit {
        debug_assert!(
            !self.is_affine() && !rhs.is_affine(),
            "cannot divide affine units: '{}' / '{}'",
            self.name,
            rhs.name
        );
        let result_unit_name = format!("{}/{}", self.name, rhs.name);
        let mut result_dimensions: DimensionMap = self.dimensions.clone();
        for (dimension, &exponent) in rhs.dimensions.iter() {
            *result_dimensions.entry(dimension.clone()).or_insert(0) -= exponent;
        }
        result_dimensions.retain(|_, &mut exp| exp != 0);
        let dimensions_vec: Vec<(Dimension, i8)> = result_dimensions.into_iter().collect();
        Unit::new(
            &result_unit_name,
            self.conversion_factor() / rhs.conversion_factor(),
            &dimensions_vec,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::quantity::Quantity;

    #[test]
    fn test_unit_creation() {
        let meter = Unit::meter();
        assert_eq!(meter.name, "meter");
        assert_eq!(meter.conversion_factor(), 1.0);
    }

    // Test that meter and foot ARE compatible (both measure length)
    // Use the is_compatible_with() method
    #[test]
    fn test_compatible_units() {
        let meter = Unit::meter();
        let foot = Unit::foot();
        assert!(meter.is_compatible_with(&foot));
        assert!(foot.is_compatible_with(&meter));
    }

    // Test that meter and second are NOT compatible (different dimensions)
    #[test]
    fn test_incompatible_units() {
        let meter = Unit::meter();
        let second = Unit::second();
        assert!(!meter.is_compatible_with(&second));
        assert!(!second.is_compatible_with(&meter));
    }

    #[test]
    fn test_unit_multiplication() {
        // Test meter * second
        let meter = Unit::meter();
        let second = Unit::second();
        let meter_second = meter * second;

        assert_eq!(meter_second.name, "meter*second");
        assert_eq!(meter_second.conversion_factor(), 1.0);

        let meter_second_dims = meter_second.dimension_string();
        assert!(
            meter_second_dims.contains("length")
                && meter_second_dims.contains("*")
                && meter_second_dims.contains("time")
        );
    }

    #[test]
    fn test_unit_division() {
        // Test meter / second (velocity)
        let meter = Unit::meter();
        let second = Unit::second();
        let velocity = meter / second;

        assert_eq!(velocity.name, "meter/second");
        assert_eq!(velocity.conversion_factor(), 1.0);
        assert_eq!(velocity.dimension_string(), "length/time");
    }

    #[test]
    fn test_compound_unit_conversion() {
        // Test km/hr to m/s
        let km = Unit::kilometer();
        let hour = Unit::hour();
        let kmh = km / hour;

        let m = Unit::meter();
        let s = Unit::second();
        let ms = m / s;

        // 1 km/hr = 1000m/3600s = 0.2778 m/s
        let speed = Quantity::new(1.0, kmh);
        let converted = speed.convert_to(&ms).unwrap();
        assert!((converted.value - 0.2778).abs() < 0.001);
    }

    #[test]
    fn test_operator_precedence() {
        // Test that a / b * c is evaluated as (a / b) * c, not a / (b * c)
        // These two expressions should give DIFFERENT results!
        let ltr = Unit::meter() / Unit::second() * Unit::kilogram();
        let with_parens = Unit::meter() / (Unit::second() * Unit::kilogram());
        let ltr_dims = ltr.dimension_string();

        assert_ne!(ltr_dims, with_parens.dimension_string());
        assert!(ltr_dims.contains("/time"));
        assert!(ltr_dims.contains("length") && ltr_dims.contains("mass"));
        assert!(with_parens.dimension_string().contains("length/"));
    }

    #[test]
    fn test_new_si_dimensions() {
        // Test the new SI base units
        let mol = Unit::mole();
        assert_eq!(mol.name, "mole");
        assert_eq!(mol.dimension_string(), "amount");

        let cd = Unit::candela();
        assert_eq!(cd.name, "candela");
        assert_eq!(cd.dimension_string(), "intensity");

        // Test angle conversion
        let rad = Unit::radian();
        let deg = Unit::degree();
        let angle = Quantity::new(180.0, deg);
        let in_radians = angle.convert_to(&rad).unwrap();
        // 180 degrees = π radians ≈ 3.14159
        assert!((in_radians.value - std::f64::consts::PI).abs() < 0.001);

        // Test information units
        let bits = Unit::bit();
        let bytes = Unit::byte();
        let data = Quantity::new(1024.0, bytes);
        let in_bits = data.convert_to(&bits).unwrap();
        assert_eq!(in_bits.value, 8192.0); // 1024 * 8
    }

    #[test]
    fn test_dimension_string() {
        // Test simple dimension
        let meter = Unit::meter();
        assert_eq!(meter.dimension_string(), "length");

        // Test velocity (length/time)
        let velocity = Unit::new(
            "velocity",
            1.0,
            &[(Dimension::Length, 1), (Dimension::Time, -1)],
        );
        assert_eq!(velocity.dimension_string(), "length/time");

        // Test acceleration (length/time^2)
        let acceleration = Unit::new(
            "acceleration",
            1.0,
            &[(Dimension::Length, 1), (Dimension::Time, -2)],
        );
        assert_eq!(acceleration.dimension_string(), "length/time^2");

        // Test force (mass*length/time^2)
        let force = Unit::new(
            "newton",
            1.0,
            &[
                (Dimension::Mass, 1),
                (Dimension::Length, 1),
                (Dimension::Time, -2),
            ],
        );
        // The order might vary since HashMap doesn't guarantee order
        // So we just check it contains the right parts
        let result = force.dimension_string();
        assert!(result.contains("mass"));
        assert!(result.contains("length"));
        assert!(result.contains("time^2"));
    }

    // ---- Temperature conversion tests ----

    #[test]
    fn test_celsius_to_kelvin() {
        let c = Unit::celsius();
        let k = Unit::kelvin();
        // 0°C = 273.15 K
        let q = Quantity::new(0.0, c.clone());
        let result = q.convert_to(&k).unwrap();
        assert!((result.value - 273.15).abs() < 1e-9);
        // 100°C = 373.15 K
        let q = Quantity::new(100.0, c);
        let result = q.convert_to(&k).unwrap();
        assert!((result.value - 373.15).abs() < 1e-9);
    }

    #[test]
    fn test_fahrenheit_to_celsius() {
        let f = Unit::fahrenheit();
        let c = Unit::celsius();
        // 32°F = 0°C
        let q = Quantity::new(32.0, f.clone());
        let result = q.convert_to(&c).unwrap();
        assert!(result.value.abs() < 1e-9);
        // 212°F = 100°C
        let q = Quantity::new(212.0, f);
        let result = q.convert_to(&c).unwrap();
        assert!((result.value - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_body_temperature() {
        let f = Unit::fahrenheit();
        let c = Unit::celsius();
        // 98.6°F = 37°C
        let q = Quantity::new(98.6, f);
        let result = q.convert_to(&c).unwrap();
        assert!((result.value - 37.0).abs() < 1e-9);
    }

    #[test]
    fn test_kelvin_to_celsius() {
        let k = Unit::kelvin();
        let c = Unit::celsius();
        // 0 K = -273.15°C
        let q = Quantity::new(0.0, k);
        let result = q.convert_to(&c).unwrap();
        assert!((result.value - (-273.15)).abs() < 1e-9);
    }

    #[test]
    fn test_rankine_to_kelvin() {
        let ra = Unit::rankine();
        let k = Unit::kelvin();
        // 0 Ra = 0 K (both absolute zero)
        let q = Quantity::new(0.0, ra.clone());
        let result = q.convert_to(&k).unwrap();
        assert!(result.value.abs() < 1e-9);
        // 491.67 Ra = 273.15 K (0°C)
        let q = Quantity::new(491.67, ra);
        let result = q.convert_to(&k).unwrap();
        assert!((result.value - 273.15).abs() < 1e-6);
    }

    #[test]
    fn test_reaumur_to_celsius() {
        let re = Unit::reaumur();
        let c = Unit::celsius();
        // 0°Ré = 0°C
        let q = Quantity::new(0.0, re.clone());
        let result = q.convert_to(&c).unwrap();
        assert!(result.value.abs() < 1e-9);
        // 80°Ré = 100°C
        let q = Quantity::new(80.0, re);
        let result = q.convert_to(&c).unwrap();
        assert!((result.value - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_temperature_round_trip() {
        let c = Unit::celsius();
        let f = Unit::fahrenheit();
        // Round-trip: 37°C → °F → °C
        let q = Quantity::new(37.0, c.clone());
        let in_f = q.convert_to(&f).unwrap();
        let back = in_f.convert_to(&c).unwrap();
        assert!((back.value - 37.0).abs() < 1e-9);
    }

    #[test]
    fn test_affine_unit_detection() {
        assert!(Unit::celsius().is_affine());
        assert!(Unit::fahrenheit().is_affine());
        assert!(Unit::reaumur().is_affine());
        assert!(!Unit::kelvin().is_affine());
        assert!(!Unit::rankine().is_affine());
        assert!(!Unit::meter().is_affine());
    }
}
