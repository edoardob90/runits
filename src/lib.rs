//! # RUnits - GNU Units-inspired unit converter in Rust
//!
//! RUnits is a powerful, type-safe unit conversion tool with support for compound units,
//! dimensional analysis, and interactive mode.
//!
//! ## Quick Start
//!
//! ```
//! use runits::{Unit, Quantity};
//! use runits::units::dimension::Dimension;
//!
//! // Create a quantity
//! let distance = Quantity::new(10.0, Unit::foot());
//!
//! // Convert to another unit
//! let meters = distance.convert_to(&Unit::meter()).unwrap();
//! println!("{}", meters); // Prints: 3.048 meter
//! ```
//!
//! ## Features
//!
//! - **Type-safe conversions**: Dimensional analysis prevents invalid conversions
//! - **Compound units**: Support for complex units like `kg*m/s^2`
//! - **Extensive unit database**: SI, Imperial, and specialized units
//! - **CLI interface**: Direct command-line unit conversions
//! - **Interactive mode**: REPL for exploratory unit calculations
//!
//! ## Project Documentation
//!
//! For additional documentation beyond the API reference:
//!
//! - **Learning Notes** - Key Rust concepts learned during development (see `docs/learning-notes.md`)
//! - **Design Decisions** - Implementation rationale and trade-offs (see `docs/design-decisions.md`)
//! - **[Development Plan](https://github.com/edoardob90/runits/blob/main/PLAN.md)** -
//!   Comprehensive development roadmap
//!
//! ## CLI Usage
//!
//! ```bash
//! # Basic conversion
//! runits "10 ft" "m"
//!
//! # Compound units
//! runits "100 km/hr" "m/s"
//!
//! # Interactive mode
//! runits
//! ```

pub mod units;

// Re-export the main types for easy importing
pub use units::{Dimension, Quantity, Unit};

// Re-export common function
// ...
