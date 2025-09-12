// This file defines what types of measurement exist (lenght, mass, energy, etc.)
// It defines the "categories" that units belong to

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// Hash is needed because we'll use Dimension as HashMap keys
// Eq is needed when you have Hash
pub enum Dimension {
    // Basic SI dimensions
    Length,
    Mass,
    Time,
    Temperature,
    Current,
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
