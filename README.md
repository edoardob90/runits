# RUnits - A GNU Units-inspired converter in Rust

RUnits is a command-line unit converter built in Rust, inspired by GNU `units`. It supports direct conversions, compound units, SI and binary prefixes, and temperature scales — all with type-safe dimensional analysis.

## Usage Examples

### Direct Conversion

```
$ runits "2.5 miles" "km"
4.02336 km
```

### Compound Units

```
$ runits "100 km/hr" "m/s"
27.7778 m/s

$ runits "1 kgf" "newton"
9.80665 newton
```

### Temperature Scales

```
$ runits "98.6 F" "C"
37 C

$ runits "0 C" "K"
273.15 K
```

### SI & Binary Prefixes

```
$ runits "1 GiB" "MiB"
1024 MiB

$ runits "5 km" "m"
5000 m
```

### Output Formatting

```
$ runits --precision 10 "1 mile" "meter"
1609.344000 meter

$ runits --scientific "1 AU" "km"
1.49598e8 km

$ runits --to-base "1 newton" "kg*m/s^2"
1 kg*m*s^-2
```

## Features
- **Compound-unit parsing**: handles `kg*m/s^2`, `km/hr`, and arbitrary combinations
- **SI prefixes** (yotta → yocto, 24 levels) and **binary prefixes** (Ki → Ei)
- **Temperature conversions**: Celsius, Fahrenheit, Kelvin, Rankine, Réaumur (affine)
- **~63 built-in units** spanning length, mass, time, force, pressure, energy, power, cooking, astronomical, radioactivity, and more
- **Type-safe dimensional analysis**: prevents nonsensical conversions (e.g., meters to seconds)
- **Output control**: `--precision`, `--scientific`, `--to-base` flags
- **Clear error messages** for unknown units, incompatible dimensions, and parse failures

### Interactive REPL

```
$ runits
  runits 0.1.0
  Unit converter with dimensional analysis

  Unit system: SI
  Database: 63 (builtin) + SI/binary prefixes
  Config: ~/.config/runits/config.toml

  Syntax: <quantity> -> <target>
  Type ? for unit help, info for status, quit to exit.

>>> 100 km/h -> m/s
27.7778 meter/second [Velocity]

>>> ? N
newton (N, newtons)
  Quantity: Force
  Dimensions: L·M·T⁻²
  SI base: kg·m·s⁻²
  Factor: 1 (reference)
  Compatible: dyne, kilogram_force, pound_force
  + SI prefixes

>>> 10 ft
10 foot [Length]
```

- Dimension-based color theme (Flexoki-inspired): Length=blue, Mass=red, Time=green, etc.
- Fish-style inline hints + syntax highlighting as you type
- Dimension-aware tab-completion (after `->` only compatible units are suggested)
- `?` queries for unit info, `quantity -> ?` to explore compatible targets
- `info` command for system status + color legend
- Configurable banner (`--intro-banner long|short|off`)
- Persistent history at `~/.config/runits/history`

### Additional Features

- **JSON output**: `runits --json "10 ft" "m"`
- **Batch mode**: `echo "10 ft -> m" | runits --batch`
- **Unicode rendering**: `runits --pretty "1 N" "N"` → `1 kg·m·s⁻²`
- **Shell completions**: `runits completions bash/zsh/fish`
- **TOML config**: `~/.config/runits/config.toml` for default precision, color, Unicode

### Planned
- User-defined units, physical constants, math expressions (Phase 5)
- GNU `definitions.units` parser with tiered loading (Phase 5)
- Configurable color themes with hex/truecolor support

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
cargo run -- "10 ft" "m"
```

5. Run Tests: Ensure all tests pass before submitting a contribution.
```
cargo test
```

## About This Project

RUnits is a learning-meets-polished-tool project — roughly 70% practical tool, 30% Rust learning vehicle.

While inspired by GNU `units`, the goal is **not** feature parity or competing with [numbat](https://numbat.dev/). The focus is on a modern implementation emphasizing:

- **Correctness**: Leveraging Rust's type system to prevent dimensional errors
- **Clarity**: Code that is easy to read, maintain, and contribute to
- **Ergonomics**: A command-line tool that is a pleasure to use

All conversion and parsing logic is a fresh Rust implementation using `pest` for grammar-based parsing and `thiserror` for error handling. See the [roadmap](docs/roadmap.md) for detailed status and plans.
