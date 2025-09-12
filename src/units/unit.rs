// This file defines what a Unit is and how to create common units

use super::dimension::{Dimension, DimensionMap, create_dimensions};

#[derive(Debug, Clone)]
pub struct Unit {
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
    pub fn meter() -> Self {
        Self::new("meter", 1.0, &[(Dimension::Length, 1)])
    }

    pub fn kilogram() -> Self {
        Self::new("kilogram", 1.0, &[(Dimension::Mass, 1)])
    }

    pub fn second() -> Self {
        Self::new("second", 1.0, &[(Dimension::Time, 1)])
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
