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

## FUTURE markers

- `FUTURE(alias-types)` — symbol vs name distinction, case-sensitivity rules
