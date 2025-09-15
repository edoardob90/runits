// This file defines what a Unit is and how to create common units

use super::dimension::{Dimension, DimensionMap, create_dimensions};
use std::ops::{Div, Mul};

#[derive(Debug, Clone)]
pub struct Unit {
    // The base name
    // Rules: no plurals, lowercase
    pub name: String,
    // How many base units this represents
    // Example: 1 foot = 0.3048 meters (if meter is the base)
    pub conversion_factor: f64,
    // What this unit measures: {Length: 1} for meters, {Mass: 1, Length: 1, Time: -2} for netwons
    pub dimensions: DimensionMap,
}

impl Unit {
    // Constructor: creates a new unit
    pub fn new(name: &str, conversion_factor: f64, dimensions: &[(Dimension, i8)]) -> Self {
        Unit {
            name: name.to_string(),
            conversion_factor,
            dimensions: create_dimensions(dimensions),
        }
    }

    // ===== Factory methods for units =====

    // ----- SI BASE UNITS -----
    // These are the fundamental units with conversion_factor = 1.0

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

    // ----- OTHER BASE UNITS (non-SI) -----

    // Angle (radian)
    pub fn radian() -> Self {
        Self::new("radian", 1.0, &[(Dimension::Angle, 1)])
    }

    // Information (bit)
    pub fn bit() -> Self {
        Self::new("bit", 1.0, &[(Dimension::Information, 1)])
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

    // Check if two units measure the same thing
    // Example: both meters and feet both have dimensions {Length: 1}
    pub fn is_compatible_with(&self, other: &Unit) -> bool {
        self.dimensions == other.dimensions
    }

    // Get a human-readable description of this unit
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
}

impl PartialEq for Unit {
    // Two units are equal if they have the same name and dimensions
    // We don't compare conversion_factor values in case there're rounding errors
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.dimensions == other.dimensions
    }
}

// Implement multiplication for units: meter * second
impl Mul for Unit {
    type Output = Unit; // The result of multiplying two Units is a Unit

    fn mul(self, rhs: Unit) -> Unit {
        // Unit multiplication as a Trait
        let result_unit_name = format!("{}*{}", self.name, rhs.name);
        // Build the result's DimensionMap
        let mut result_dimensions: DimensionMap = self.dimensions.clone();
        for (dimension, &exponent) in rhs.dimensions.iter() {
            // Why the deref operator (*) here?
            // entry().or_insert(0) returns &mut i8 (a mutable reference to the value)
            // We need to update the entry's key (exponent) by summing it to the rhs
            // So we need to dereference the pointer and get its value
            *result_dimensions.entry(dimension.clone()).or_insert(0) += exponent;
        }
        // Remove the dimensions with 0 exponents
        result_dimensions.retain(|_, &mut exp| exp != 0);
        // Build a slice of tuples from the DimensionMap
        let dimensions_vec: Vec<(Dimension, i8)> = result_dimensions
            .into_iter() // Returns an iterator that yields (Dimension, i8)
            .collect(); // Gathers all items from the iterator into that collection type
        // Return the new unit
        Unit::new(
            &result_unit_name,
            self.conversion_factor * rhs.conversion_factor,
            &dimensions_vec,
        )
    }
}

// Implement division for units: meter / second = m/s
impl Div for Unit {
    type Output = Unit;

    fn div(self, rhs: Unit) -> Unit {
        // Unit division as a Trait
        let result_unit_name = format!("{}/{}", self.name, rhs.name);
        let mut result_dimensions: DimensionMap = self.dimensions.clone();
        for (dimension, &exponent) in rhs.dimensions.iter() {
            *result_dimensions.entry(dimension.clone()).or_insert(0) -= exponent;
        }
        result_dimensions.retain(|_, &mut exp| exp != 0);
        let dimensions_vec: Vec<(Dimension, i8)> = result_dimensions.into_iter().collect();
        Unit::new(
            &result_unit_name,
            self.conversion_factor / rhs.conversion_factor,
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
        assert_eq!(meter.conversion_factor, 1.0);
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
        assert_eq!(meter_second.conversion_factor, 1.0);

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
        assert_eq!(velocity.conversion_factor, 1.0);
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
}
