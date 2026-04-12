# database/ module

In-memory unit registry with O(1) lookup, dynamic SI/binary prefix stripping, and fuzzy suggestions.

## Architecture

- **`mod.rs`** — `UnitDatabase` struct + all access methods (lookup, suggest, compatible_units, aliases_for, unit_names, len). `global()` singleton via `OnceLock`. All tests.
- **`seed.rs`** — `seed_all()` function + `add()`/`rename()` helpers. Pure data: defines ~63 builtin units with aliases. No logic beyond registration.

## Key patterns

- `global()` singleton via `OnceLock` — process-wide, thread-safe, lazy init
- `try_prefix_strip()` checks `base_unit.prefixable` guard — only `Unit::new_si()` units accept prefixes
- Direct lookup always wins over prefix-derived (e.g., "min" → minute, not milli + in)
- `SI_PREFIXES` is `pub` (shared with `format.rs` for prefix detection); `BINARY_PREFIXES` is private

## Adding units

Add to `seed_all()` in `seed.rs` using the `add()` helper:
- `Unit::new(name, factor, dims)` — standard unit
- `Unit::new_si(name, factor, dims)` — prefixable (accepts SI prefix stripping)
- `Unit::new_affine(name, scale, offset, dims)` — temperature scales

## Constants database

- **`constants.rs`** — `ConstantsDB` struct with 15 CODATA physical constants, `global()` singleton, `lookup()`/`suggest()` (Jaro-Winkler, 0.7 threshold)
- Constants are tried *after* units during identifier resolution in the expression evaluator

## TDD patterns

Follow red/green/refactor — write the test before touching `seed.rs` or `mod.rs`.

- **New unit registration** — test `db.lookup("name")` returns `Some(unit)` with the correct factor and dimensions *before* adding the `add()` call to `seed.rs`. The test should fail with `None` until the registration lands.
- **New prefix behavior** — test `db.try_prefix_strip("k<unit>")` in isolation; confirm the stripped `(prefix, base_unit)` pair before adding the prefix to `SI_PREFIXES` or similar.
- **New `suggest()` threshold** — test with deliberate typos (e.g., `"metr"`, `"kilgram"`) to verify fuzzy matches appear; confirm the test fails before tuning the Jaro-Winkler threshold.
- **Constants lookups** — test `ConstantsDB::global().lookup("c")` returns a `Quantity` with the right value and dimensions before adding the constant to `constants.rs`.

Concretely: write the `#[test]` block, run `cargo test -- <test_name>` to see it fail, then make it pass with the implementation.

## FUTURE markers

- `FUTURE(alias-types)` — symbol vs name distinction, case-sensitivity rules
