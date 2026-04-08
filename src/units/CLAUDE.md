# units/ module

Core domain types: `Dimension`, `Unit`, `Quantity`.

## Architecture

- **`dimension.rs`** — `Dimension` enum (10 variants), `DimensionMap` type alias, `name()`/`analysis_symbol()`/`base_symbol()` methods, `ALL` constant
- **`unit.rs`** — `Unit` struct, `ConversionKind` enum, constructors, factory methods, `Mul`/`Div` operator impls, dimension rendering
- **`quantity.rs`** — `Quantity` struct, `convert_to()`, `format_value()`/`format_value_inner()`
- **`mod.rs`** — re-exports `Dimension`, `Unit`, `Quantity`

## Dependency chain

`dimension.rs` -> `unit.rs` -> `quantity.rs` (acyclic, each depends only on prior)

## Key patterns

- `ConversionKind::Linear(f64)` vs `Affine { scale, offset }` — affine units cannot appear in compound expressions (`debug_assert` in Mul/Div)
- `Unit.prefixable: bool` — only `new_si()` sets true; prefix-derived units get `prefixable = false`
- `render_dimensions()` — shared helper for flat notation (positive exponents first, then alphabetical by symbol); used by `dimension_string()`, `to_base_unit_string()`, `analysis_string()`
- Factory methods (`meter()`, `kilogram()`, etc.) live in `unit.rs` near the struct — not extracted, intentionally

## FUTURE markers

- `FUTURE(alias-types)` — when units carry symbol vs name metadata
