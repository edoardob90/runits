// src/lib.rs
// This makes the project a binary and a library

pub mod units;

// Re-export the main types for easy importing
pub use units::{Dimension, Quantity, Unit};

// Re-export common function
// ...
