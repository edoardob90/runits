# Learning Notes: Rust Concepts in RUnits

This document tracks key Rust concepts learned while building RUnits.

## Phase 1: Core Data Structures

### Ownership and Borrowing
- **Move semantics**: Division operator (`/`) consumes both operands
- **Cloning vs. Referencing**: When to use `.clone()` vs `&` references
- **HashMap entry API**: Using `.entry().or_insert()` for safe HashMap updates

### Type System
- **Custom types**: Defining `struct` and `enum` with proper traits
- **Trait derivation**: `#[derive(Debug, Clone, PartialEq, Eq, Hash)]`
- **Method resolution**: How inherent methods vs trait methods are resolved

### Error Handling
- **Result types**: Using `Result<T, E>` for fallible operations
- **Custom errors**: Defining application-specific error types
- **Error propagation**: The `?` operator for clean error handling

### Collections and Iteration
- **HashMap**: Key-value storage with proper key traits (Hash, Eq)
- **Iterator patterns**: `.iter()`, `.cloned()`, `.collect()`
- **Entry API**: Safe HashMap manipulation with `.entry().or_insert()`

### Documentation
- **rustdoc**: Using `///` and `//!` for comprehensive documentation
- **Cross-references**: Linking types with `[`Type`]` syntax
- **Testable examples**: Documentation that doubles as tests

## Key Insights

### 1. HashMap Iteration Gotcha
```rust
for (dimension, &exponent) in self.dimensions.iter() {
    // Note: iter() gives (&Key, &Value), hence &exponent
}
```

### 2. Operator Traits and Ownership
```rust
impl Mul for Unit {
    fn mul(self, rhs: Unit) -> Unit {
        // Takes ownership of both self and rhs
    }
}
```

### 3. Documentation Path Resolution
- Use full crate paths in examples: `runits::units::unit::Unit`
- Cross-references work with imported types: `[`DimensionMap`]`
- Examples are compiled and tested automatically

## Next Learning Goals
- Command-line parsing with `clap`
- Parser generators with `pest`
- File I/O and configuration management
- Trait objects and dynamic dispatch