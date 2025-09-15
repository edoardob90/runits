// This file defines what types of measurement exist (lenght, mass, energy, etc.)
// It defines the "categories" that units belong to

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// Hash is needed because we'll use Dimension as HashMap keys
// Eq is needed when you have Hash
pub enum Dimension {
    // Basic SI dimensions
    Length,            // meter (m)
    Mass,              // kilogram (kg)
    Time,              // second (s)
    Temperature,       // kelvin (K)
    Current,           // ampere (A)
    AmountOfSubstance, // mole (mol)
    LuminousIntensity, // candela (cd)

    // Additional useful dimensions
    Angle,       // radian (rad) - technically dimensionless, but useful to have
    Information, // bit/byte
    Currency,    // for monetary conversion and related (e.g. price of raw materials)
}

impl Dimension {
    // Helper function fo get all basic dimensions
    pub fn base_dimensions() -> Vec<Dimension> {
        vec![
            Dimension::Length,
            Dimension::Mass,
            Dimension::Time,
            Dimension::Temperature,
            Dimension::Current,
            Dimension::AmountOfSubstance,
            Dimension::LuminousIntensity,
        ]
    }

    // Helper to get a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Dimension::Length => "length",
            Dimension::Mass => "mass",
            Dimension::Time => "time",
            Dimension::Temperature => "temperature",
            Dimension::Current => "current",
            Dimension::AmountOfSubstance => "amount",
            Dimension::LuminousIntensity => "intensity",
            Dimension::Angle => "angle",
            Dimension::Information => "information",
            Dimension::Currency => "currency",
        }
    }
}

// Type alias to make the code more readable
// Instead of HashMap<Dimension, i8> we can write DimensionMap
pub type DimensionMap = HashMap<Dimension, i8>;

// Helper to create a dimension easily
pub fn create_dimensions(dimensions: &[(Dimension, i8)]) -> DimensionMap {
    // Convert a slice of tuples into a HashMap
    // Example: create_dimensions(&[(Dimension::Length, 1)])
    dimensions.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_creation() {
        // Test a basic dimension creation
        let length = Dimension::Length;
        assert_eq!(length.name(), "length");
    }

    #[test]
    fn test_map_dimension_creation() {
        // Test the helper function
        // Create a Length dimension and check that we can't return a Mass
        let dims = create_dimensions(&[(Dimension::Length, 1)]);
        assert_eq!(dims.get(&Dimension::Length), Some(&1));
        assert_eq!(dims.get(&Dimension::Mass), None);
    }

    #[test]
    fn test_compound_dimension_velocity() {
        let dims = create_dimensions(&[(Dimension::Length, 1), (Dimension::Time, -1)]);
        assert_eq!(dims.get(&Dimension::Length), Some(&1));
        assert_eq!(dims.get(&Dimension::Time), Some(&-1));
        assert_eq!(dims.get(&Dimension::Mass), None);
    }
}
