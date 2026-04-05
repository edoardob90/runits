# RUnits — Claude Code Configuration

## Project Overview

**RUnits** is a GNU Units-inspired command-line unit converter in Rust. Focus: type-safe dimensional analysis, compound units, and a pleasant CLI/REPL experience.

### Key features (target)
- Direct unit conversions (`runits "2.5 miles" "km"`)
- Compound unit parsing (`100 km/hr` → `m/s`)
- Interactive REPL mode
- Type-safe dimensional analysis
- Extensive unit database

## Project Structure

```
runits/
├── Cargo.toml             # project manifest
├── README.md              # user-facing overview
├── CLAUDE.md              # this file
├── LICENSE                # MIT
├── .github/workflows/     # CI: docs build + deploy to GitHub Pages
├── docs/
│   ├── README.md          # docs index
│   ├── roadmap.md         # status, phases, feature catalog (source of truth)
│   └── learning-notes.md  # Rust concepts learned
└── src/
    ├── lib.rs             # crate root, re-exports, embeds roadmap.md
    ├── main.rs            # CLI entry (currently demo code)
    └── units/
        ├── mod.rs         # module re-exports
        ├── dimension.rs   # Dimension enum + DimensionMap
        ├── unit.rs        # Unit struct, arithmetic, factory methods
        └── quantity.rs    # Quantity struct, conversion, errors
```

For status, next phases, and the feature catalog, see **[`docs/roadmap.md`](docs/roadmap.md)**.

## Development Environment
- **Language:** Rust (edition 2024)
- **Toolchain:** rustc 1.89.0, cargo 1.89.0

## Build & Development Commands

```bash
cargo check                          # compile check
cargo build                          # debug build
cargo build --release                # release build
cargo run                            # run demo
cargo run -- "10 ft" "m"             # run with args (future CLI)
cargo test                           # all tests (unit + doc + integration)
cargo test --doc                     # doc tests only
cargo doc --open                     # generate + open API docs
cargo doc --document-private-items   # include private items
cargo fmt --check                    # check formatting
cargo clippy                         # lint
```

## Future Dependencies (per roadmap)

- `clap` — CLI argument parsing (Phase 2)
- `thiserror` — error derive macros (Phase 2)
- `pest` — parser generator (Phase 2)
- `rustyline` — interactive REPL (Phase 4)
- `owo-colors`, `strsim`, `clap_complete` — UX polish (Phase 4)
- `serde` + `toml` — config file (Phase 4)

Full feature catalog and phase affinity in `docs/roadmap.md`.

## Code Style & Conventions

- Standard formatting: `cargo fmt`
- Clippy-clean: `cargo clippy -- -D warnings`
- Prefer `Result` over panics for recoverable failures
- Leverage the type system for dimensional safety (the core thesis of the project)
- Factory methods (`Unit::meter()`) for common constructions
- Doc-test every public API example

## Testing Strategy
- Unit tests alongside implementation (`#[cfg(test)] mod tests`)
- Doc tests in rustdoc examples
- Integration tests under `tests/` for CLI behavior (Phase 2+)
- Property-based tests for round-trip conversions (optional, see deferred track in roadmap)

## Project Instructions
- This is a **learning project**. When writing code on behalf of the user, favor small, focused changes and liberal comments over end-to-end blast-through implementations. Give tasks rather than solutions when the user is actively learning a concept.
- When the user asks for code review, check understanding by asking targeted questions about *why* particular choices were made.
