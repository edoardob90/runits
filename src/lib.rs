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
//! - **[Roadmap](roadmap/index.html)** - Status, next phases, and feature catalog
//! - **Learning Notes** - Key Rust concepts learned during development (see `docs/learning-notes.md`)
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

pub mod annotations;
pub mod cli;
pub mod database;
pub mod error;
pub mod parser;
pub mod units;

pub use error::RUnitsError;

/// Project roadmap: status, next phases, and feature catalog.
///
/// See [`docs/roadmap.md`](https://github.com/edoardob90/runits/blob/main/docs/roadmap.md)
/// for the source. This module exists only to surface the roadmap inside the
/// generated rustdoc site.
#[doc = include_str!("../docs/roadmap.md")]
pub mod roadmap {}

// Re-export the main types for easy importing
pub use units::{Dimension, Quantity, Unit};
