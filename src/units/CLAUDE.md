# units/ module

Core domain types: `Dimension`, `Unit`, `Quantity`.

## Architecture

- **`dimension.rs`** — `Dimension` enum (10 variants), `DimensionMap` type alias, `name()`/`analysis_symbol()`/`base_symbol()` methods, `ALL` constant
- **`unit.rs`** — `Unit` struct, `ConversionKind` enum, constructors, factory methods, `Mul`/`Div` operator impls, dimension rendering
- **`quantity.rs`** — `Quantity` struct, `convert_to()`, `format_value()`/`format_value_inner()`, fallible arithmetic (`try_add`/`try_sub`/`mul`/`div`/`pow_i32`/`neg`)
- **`mod.rs`** — re-exports `Dimension`, `Unit`, `Quantity`

## Dependency chain

`dimension.rs` -> `unit.rs` -> `quantity.rs` (acyclic, each depends only on prior)

## Key patterns

- `ConversionKind::Linear(f64)` vs `Affine { scale, offset }` — affine units cannot appear in compound expressions (`debug_assert` in Mul/Div) or quantity arithmetic (`AffineInExpression` error)
- Quantity arithmetic returns `Result` — dimensional checking on add/sub, affine rejection on all ops
- `Unit.prefixable: bool` — only `new_si()` sets true; prefix-derived units get `prefixable = false`
- `render_dimensions()` — shared helper for flat notation (positive exponents first, then alphabetical by symbol); used by `dimension_string()`, `to_base_unit_string()`, `analysis_string()`
- Factory methods (`meter()`, `kilogram()`, etc.) live in `unit.rs` near the struct — not extracted, intentionally

## TDD patterns

Follow red/green/refactor — write the test before touching `dimension.rs`, `unit.rs`, or `quantity.rs`.

- **New `Dimension` variant** — test that `DimensionMap` correctly accumulates the new dimension in arithmetic results, and that `analysis_symbol()`/`base_symbol()` return the right strings; confirm failure before adding the variant.
- **New `Unit` constructor or factory method** — test the round-trip: construct the unit, convert a `Quantity` through `convert_to()`, assert the resulting value and dimension string. The test fails until the constructor is wired to the right `ConversionKind`.
- **New `Quantity` arithmetic** — test `try_add`/`try_sub`/`mul`/`div`/`pow_i32` with compatible and incompatible dimensions. The incompatible-dimension test must see a `DimensionMismatch` error *before* adding the operation; the compatible test must fail with the wrong value or panic before the implementation is correct.
- **Affine rejection** — test that building a compound unit with an affine unit returns `AffineInExpression`; confirm the test fails (no error returned) before adding the `debug_assert`/guard in `Mul`/`Div`.

Concretely: run `cargo test -- units::` after writing the test to confirm red, then implement.

## FUTURE markers

- `FUTURE(alias-types)` — when units carry symbol vs name metadata
