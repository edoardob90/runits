# RUnits - Claude Code Configuration

## Project Overview
**RUnits** is a GNU Units-inspired command-line unit converter built in Rust. The project focuses on creating a powerful, type-safe unit conversion tool with support for compound units, dimensional analysis, and interactive mode.

### Key Features
- Direct unit conversions (`runits "2.5 miles" "km"`)
- Compound unit parsing (`100 km/hr` to `m/s`)
- Interactive REPL mode
- Type-safe dimensional analysis
- Comprehensive unit database support

## Project Structure
```
runits/
├── Cargo.toml          # Rust project configuration
├── README.md           # Project documentation
├── PLAN.md            # Detailed 5-phase development plan
├── LICENSE            # MIT license
├── .gitignore         # Git ignore rules
└── src/
    └── main.rs        # Entry point (currently "Hello, world!")
```

## Development Environment
- **Language:** Rust (edition 2024)
- **Toolchain:** rustc 1.89.0, cargo 1.89.0
- **Target:** Command-line application
- **Current State:** Early development (Phase 0 - basic project setup)

## Build & Development Commands
```bash
# Check compilation without building
cargo check

# Build the project
cargo build

# Build for release
cargo build --release

# Run the application
cargo run

# Run with arguments (for unit conversion)
cargo run -- "10 ft" "m"

# Run tests
cargo test

# Run in interactive mode
cargo run
```

## Development Phases (from PLAN.md)

### Phase 1: Core Data Structures & Logic
- Define `Dimension` enum (Length, Mass, Time, etc.)
- Create `Unit` struct with conversion factors and dimensions
- Create `Quantity` struct (value + unit)
- Implement dimensional analysis and conversion logic
- Custom error handling with `thiserror`

**Key Rust Concepts:** structs, enums, HashMap, ownership/borrowing, Result types

### Phase 2: CLI Interface
- Integrate `clap` for command-line argument parsing
- Create basic parser for simple units
- Implement main CLI flow
- Error handling and user feedback

**Key Rust Concepts:** crates/modules, derive macros, String vs &str, error propagation (?)

### Phase 3: Advanced Parser
- Implement `pest` parser for compound units
- Define formal grammar for unit expressions
- Parse complex expressions like `kg*m/s^2`
- Tree walking and recursive parsing

**Key Rust Concepts:** parsing, pest crate, match expressions, recursion

### Phase 4: Unit Database
- Create `UnitDatabase` struct with HashMap
- Load comprehensive unit definitions
- Parse GNU `units.dat` file format
- Integration with parser

**Key Rust Concepts:** File I/O, static/lazy_static, lifetimes

### Phase 5: Polish & Advanced Features
- Interactive REPL with `rustyline`
- Improved error messages and suggestions
- Additional features (currency, prefixes)

**Key Rust Concepts:** closures, traits, conditional compilation

## Dependencies (Future)
Based on the development plan, the project will likely use:
- `clap` - Command-line argument parsing
- `thiserror` - Custom error types
- `pest` - Parser generator for unit expressions
- `lazy_static` or `once_cell` - Static initialization
- `rustyline` - Interactive REPL
- `serde` - Potentially for configuration files

## Code Style & Conventions
- Follow standard Rust formatting (`cargo fmt`)
- Use `clippy` for linting (`cargo clippy`)
- Prefer `Result` types for error handling
- Use meaningful type names (e.g., `Quantity`, `Dimension`)
- Leverage Rust's type system for dimensional safety

## Testing Strategy
- Unit tests for core conversion logic
- Integration tests for CLI interface
- Parser tests for various unit expressions
- Error handling tests

## Current Status
The project is in its initial state with only a basic "Hello, world!" implementation. Ready to begin Phase 1 development of core data structures and conversion logic.