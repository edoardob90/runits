// This file defines what a Unit is and how to create common units

use super::dimension::{Dimension, DimensionMap, create_dimensions};

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

    // Factory methods for common units
    // Length units
    pub fn meter() -> Self {
        Self::new("meter", 1.0, &[(Dimension::Length, 1)])
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

    pub fn kilometer() -> Self {
        Self::new("kilometer", 1000.0, &[(Dimension::Length, 1)])
    }

    // Mass units
    pub fn kilogram() -> Self {
        Self::new("kilogram", 1.0, &[(Dimension::Mass, 1)])
    }

    // Time units
    pub fn second() -> Self {
        Self::new("second", 1.0, &[(Dimension::Time, 1)])
    }

    pub fn minute() -> Self {
        Self::new("minute", 60.0, &[(Dimension::Time, 1)])
    }

    // Check if two units measure the same thing
    // Example: both meters and feet both have dimensions {Length: 1}
    pub fn is_compatible_with(&self, other: &Unit) -> bool {
        self.dimensions == other.dimensions
    }

    // Get a human-readable description of this unit
    pub fn dimension_string(&self) -> String {
        // Convert {Length: 1, Time: -1} into "length/time"
        todo!("Implement the dimension string formatting")
    }
}

impl PartialEq for Unit {
    // Two units are equal if they have the same name and dimensions
    // We don't compare conversion_factor values in case there're rounding errors
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.dimensions == other.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
