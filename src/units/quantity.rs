// This file defines a Quantity, i.e., a number with a unit
// This is the core data structure to represent a physical quantity

use super::unit::Unit;
use std::fmt;

// Custom error type for conversion errors
#[derive(Debug, Clone)]
pub enum ConversionError {
    IncompatibleDimensions { from_unit: String, to_unit: String },
    // Add more error types when needed
}

// This lets us use ? operator and print errors nicely
impl std::error::Error for ConversionError {}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionError::IncompatibleDimensions { from_unit, to_unit } => {
                write!(
                    f,
                    "Cannot convert from {} to {} - incompatible dimensions",
                    from_unit, to_unit
                )
            }
        }
    }
}

// The Quantity struct represents a physical quantity with a value and a unit
#[derive(Debug, Clone)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

impl Quantity {
    // Constructor
    pub fn new(value: f64, unit: Unit) -> Self {
        Quantity { value, unit }
    }

    // Conversion function - this is where the "magic" happens!
    pub fn convert_to(&self, target_unit: &Unit) -> Result<Quantity, ConversionError> {
        // Step 1: Check if conversion is possible
        if !self.unit.is_compatible_with(target_unit) {
            return Err(ConversionError::IncompatibleDimensions {
                from_unit: self.unit.name.clone(),
                to_unit: target_unit.name.clone(),
            });
        }

        // Step 2: Do the math
        // Convert to base units first, then to target
        // Example: 5 miles -> (5 * 1.610) meters -> (8.05 / 1.0) meters = 8.05 meters
        let base_value = self.value * self.unit.conversion_factor;
        let target_value = base_value / target_unit.conversion_factor;

        Ok(Quantity::new(target_value, target_unit.clone()))
    }

    // Helper function for easy conversion (returns just the number)
    pub fn convert_value_to(&self, target_unit: &Unit) -> Result<f64, ConversionError> {
        self.convert_to(target_unit).map(|q| q.value)
    }

    // Get a nice string representation
    pub fn to_string(&self) -> String {
        format!("{} {}", self.value, self.unit.name)
    }
}

// Helper function to convert between units
pub fn convert_quantity(
    value: f64,
    from_unit: &Unit,
    to_unit: &Unit,
) -> Result<f64, ConversionError> {
    let quantity = Quantity::new(value, from_unit.clone());
    quantity.convert_value_to(to_unit)
}

// Factory functions for common quantities
impl Quantity {
    pub fn meters(value: f64) -> Self {
        Self::new(value, Unit::meter())
    }

    pub fn seconds(value: f64) -> Self {
        Self::new(value, Unit::second())
    }

    pub fn kilograms(value: f64) -> Self {
        Self::new(value, Unit::kilogram())
    }
}
