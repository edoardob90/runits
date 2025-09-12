# RUnits - A GNU Units-inspired Converter in Rust

RUnits is a powerful and user-friendly command-line unit converter built in Rust, inspired by the legendary GNU `units` program. It supports direct conversions, compound units, and an interactive mode for complex queries.

This project is built with a focus on clean code, robust error handling, and an extensible architecture, making it a great example of idiomatic Rust.

## Usage Examples

### Direct Conversion
Provide the quantity to convert and the desired target unit.

```
$ runits "2.5 miles" "km"
> 4.02336 km
```

### Compound & Complex Units
RUnits can parse and convert compound units involving multiplication, division, and exponents.

```
$ runits "100 km/hr" "m/s"
> 27.7777... m/s

$ runits "2000 revolutions/minute" "rad/s"
> 209.4395... rad/s
```

### Interactive Mode (REPL)

For multiple conversions, you can start the interactive mode by running `runits` with no arguments.

```
$ runits
RUnits Interactive Mode. Press Ctrl+D to exit.
You have: 2 lightyears
You want: parsecs
> 0.61346 pc
You have: 150 pounds * 9.8 m/s^2
You want: newtons
> 667.233 N
You have:

```

## Features
- **Simple & Compound Unit Parsing**: Handles units like kilograms, m/s, and kg*m/s^2.
- **Extensive Unit Database**: Powered by a parser for the comprehensive GNU `units.dat` file.
- **Interactive REPL**: For quick, successive conversions.
- **Type-Safe Logic**: Prevents nonsensical conversions (e.g., meters to seconds).
- **Clear Error Messages**: Provides helpful feedback for typos or invalid operations.

## Contributing & Setup

Contributions are welcome! Whether it's a bug fix, a feature suggestion, or a documentation improvement, feel free to open an issue or pull request.

### Setup for Development

1. Install Rust: Ensure you have the latest stable Rust toolchain installed via rustup.
2. Clone the Repository:
```
git clone https://github.com/edoardob90/runits.git
cd runits
```
3. Build the Project: `cargo build --release`

The binary will be located at target/release/runits.

4. Run the Application: You can run the application directly using Cargo:
```
# Example direct conversion
cargo run -- "10 ft" "m"

# Example interactive mode
cargo run
```

5. Run Tests: Ensure all tests pass before submitting a contribution.
```
cargo test
```

## About This Project

RUnits began as a learning project to deeply explore core Rust concepts through a practical, real-world application. The primary goal was to build a tool that is both useful and a clear example of high-quality Rust code.

While inspired by GNU units, the goal is **not** to achieve 1:1 feature parity, or compete with a much more advanced calculator like [numbat](https://numbat.dev/). Instead, the focus is on a modern implementation with an emphasis on:

- **Correctness**: Leveraging Rust's type system to prevent dimensional errors.
- **Clarity**: Writing code that is easy to read, maintain, and contribute to.
- **Ergonomics**: Creating a command-line tool that is a pleasure to use.

The project relies on parsing the standard GNU `units.dat` file for its extensive database, but all conversion and parsing logic is a fresh implementation in Rust.
