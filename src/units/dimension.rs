//! Physical dimensions for unit conversion and dimensional analysis.
//!
//! This module defines the fundamental types of measurement (length, mass, time, etc.)
//! that form the basis of the unit system. Each dimension represents a category
//! that units belong to, enabling type-safe conversions and dimensional analysis.

use std::collections::HashMap;

/// Represents a fundamental physical dimension.
///
/// Dimensions are the categories that units belong to - for example, both meters
/// and feet belong to the [`Length`](Dimension::Length) dimension, making them
/// compatible for conversion.
///
/// # Examples
/// ```
/// use runits::units::dimension::Dimension;
///
/// let length = Dimension::Length;
/// let mass = Dimension::Mass;
///
/// assert_eq!(length.name(), "length");
/// assert_eq!(mass.name(), "mass");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// Spatial extent - SI base unit: meter (m)
    Length,
    /// Inertial property - SI base unit: kilogram (kg)
    Mass,
    /// Duration - SI base unit: second (s)
    Time,
    /// Thermodynamic temperature - SI base unit: kelvin (K)
    Temperature,
    /// Electric current - SI base unit: ampere (A)
    Current,
    /// Amount of substance - SI base unit: mole (mol)
    AmountOfSubstance,
    /// Luminous intensity - SI base unit: candela (cd)
    LuminousIntensity,

    /// Planar angle - base unit: radian (rad)
    ///
    /// Technically dimensionless in SI, but useful to distinguish
    /// angular measurements from pure numbers.
    Angle,
    /// Digital information - base unit: bit
    Information,
    /// Monetary value - for currency conversion
    ///
    /// Note: Currency conversions require external exchange rate data.
    Currency,
}

impl Dimension {
    /// Returns all seven SI base dimensions.
    ///
    /// This excludes derived dimensions like [`Angle`](Dimension::Angle),
    /// [`Information`](Dimension::Information), and [`Currency`](Dimension::Currency).
    ///
    /// # Examples
    /// ```
    /// use runits::units::dimension::Dimension;
    ///
    /// let base_dims = Dimension::base_dimensions();
    /// assert_eq!(base_dims.len(), 7);
    /// assert!(base_dims.contains(&Dimension::Length));
    /// assert!(base_dims.contains(&Dimension::Mass));
    /// ```
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

    /// Returns a human-readable name for this dimension.
    ///
    /// # Examples
    /// ```
    /// use runits::units::dimension::Dimension;
    ///
    /// assert_eq!(Dimension::Length.name(), "length");
    /// assert_eq!(Dimension::AmountOfSubstance.name(), "amount");
    /// assert_eq!(Dimension::LuminousIntensity.name(), "intensity");
    /// ```
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

/// Type alias for dimension maps used in unit definitions.
///
/// Maps each [`Dimension`] to its exponent in a unit's dimensional formula.
/// For example, velocity (m/s) would be `{Length: 1, Time: -1}`.
pub type DimensionMap = HashMap<Dimension, i8>;

/// Creates a [`DimensionMap`] from a slice of (dimension, exponent) pairs.
///
/// This is a convenience function for building the dimensional formula
/// of a unit from a list of dimensions and their exponents.
///
/// # Examples
/// ```
/// use runits::units::dimension::{create_dimensions, Dimension};
///
/// // Create velocity dimensions: length/time
/// let velocity_dims = create_dimensions(&[
///     (Dimension::Length, 1),
///     (Dimension::Time, -1)
/// ]);
///
/// assert_eq!(velocity_dims.get(&Dimension::Length), Some(&1));
/// assert_eq!(velocity_dims.get(&Dimension::Time), Some(&-1));
/// ```
pub fn create_dimensions(dimensions: &[(Dimension, i8)]) -> DimensionMap {
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
