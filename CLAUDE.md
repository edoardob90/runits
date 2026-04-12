# RUnits — Claude Code Configuration

## Project Overview

**RUnits** is a GNU Units-inspired command-line unit converter in Rust. Focus: type-safe dimensional analysis, compound units, and a pleasant CLI/REPL experience.

### Key features (target)
- Direct unit conversions (`runits "2.5 miles" "km"`)
- Compound unit parsing (`100 km/hr` → `m/s`)
- Interactive REPL mode
- Type-safe dimensional analysis
- Extensive unit database

---

## Working with the Roadmap (READ FIRST)

**[`docs/roadmap.md`](docs/roadmap.md) is the single source of truth** for what is done, what is active, and what comes next.

### Non-negotiable workflow rules

1. **Check the roadmap before proposing or starting any change.** Every suggestion — a new feature, a refactor, a dependency, a test strategy, a file reorganization — must be reconciled with the current roadmap before action. If it already exists in the roadmap: use the existing scope, phase, and rationale. If it doesn't: stop and evaluate whether it belongs there.

2. **If a change diverges from the roadmap, update the roadmap *first*.** Do not silently go off-plan. When you (or Claude) believe something is worth doing differently than what the roadmap says — different scope, different ordering, different design, a new feature, a dropped feature — the first action is to open a proposal against the roadmap, discuss, and commit the roadmap update **before** any code change lands. Roadmap edit → approval → code edit. Never the reverse.

3. **Keep the roadmap's Status section current.** When a phase completes, update the Status table (phase → ✅ Complete, next phase → ⏳ Active). When a design decision is made that invalidates a phase's stated scope, update that phase's section. The roadmap must always reflect the **as-of-now** state of the project plus the next-up phases — never a stale snapshot.

4. **Small roadmap edits are fine and expected.** Tightening a phase's scope, promoting a catalog item into a phase, recording a design decision — these are routine. The rule is about *synchronizing the doc with reality*, not about bureaucratic ceremony.

### Quick self-check before making a change
- [ ] I've read the relevant section of `docs/roadmap.md`.
- [ ] The change I'm about to make fits the current phase's scope, OR I am updating the roadmap first.
- [ ] After the change, the Status section and any affected phase section will still be accurate.
- [ ] I've written failing test(s) *before* writing any implementation code (TDD red phase).
- [ ] I've confirmed the tests fail for the right reason — meaningful assertion error, not a compile error.
- [ ] After implementing, `cargo test` is green and the new test count is reflected in the Status section.

---

## Plan Lifecycle & Verification

Every plan Claude writes (plan mode or otherwise) **must** include three mandatory sections, in order:

### 1. Red Phase — tests to write first

List every test to write before touching implementation code. For each test, state:
- Where it lives (source file + module path)
- What it asserts (inputs → expected output / error variant)
- The expected failure message when it runs red

These tests must compile and fail *before* any implementation lands. If a test can't compile yet (e.g., the function doesn't exist), add the minimum stub (`todo!()` / `unimplemented!()`) so the test fails at runtime with a real assertion error, not a build error.

### 2. Implementation steps

Code changes that make the red tests pass. No implementation code before the tests are red.

### 3. User Verification Steps

Concrete shell commands the user runs to confirm "done and working". Cover the happy path + at least 2–3 key failure modes the plan introduces.

**Claude's self-tests are table stakes, not verification.** Claude runs `cargo check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo fmt --check` after every change and reports pass/fail. The plan is only *done* when (1) Claude's self-tests pass, AND (2) the user runs the listed verification steps and confirms they pass.

**After user confirms:** Claude updates `docs/roadmap.md` Status table (phase → ✅ Complete) and the test-suite count in the Status section, then commits.

---

## Project Structure

Module-specific instructions:
- [Database](src/database/CLAUDE.md)
- [REPL](src/repl/CLAUDE.md)
- [Units](src/units/CLAUDE.md)
 
Load these instructions automatically when working in those directories.

```
runits/
├── Cargo.toml             # project manifest
├── README.md              # user-facing overview
├── CLAUDE.md              # this file (root instructions)
├── LICENSE                # MIT
├── .github/workflows/     # CI: docs build + deploy to GitHub Pages
├── docs/
│   ├── README.md            # docs index
│   ├── roadmap.md           # source of truth: status, phases, feature catalog
│   ├── gnu-units-parity.md  # feature gap analysis vs GNU Units
├── tests/
│   └── cli_tests.rs       # assert_cmd integration tests
└── src/
    ├── lib.rs             # crate root, re-exports
    ├── main.rs            # CLI entry point + dispatch (one-shot/REPL/batch/completions)
    ├── cli.rs             # clap-derived Cli struct + Commands subcommand enum
    ├── parser.rs          # pest parser + compound-unit expression tree walker
    ├── expr.rs            # Expr AST enum (Number/Ident/Previous/BinOp/Neg/Pow/FuncCall) + pest→AST builder
    ├── eval.rs            # EvalContext tree walker: identifier resolution, binop dispatch, function calls
    ├── math.rs            # MathFn enum-dispatch registry (sqrt/sqr/abs/sin/cos/tan/ln/exp)
    ├── error.rs           # unified error enum via thiserror (with fuzzy suggestions)
    ├── annotations.rs     # dimension-signature → physical-quantity name registry
    ├── convert.rs         # ConversionResult + run_conversion() (shared by CLI/REPL/batch)
    ├── theme.rs           # Theme struct (Flexoki-inspired dimension-based colors), paint/style methods
    ├── format.rs          # FormatOptions, format_result/unit_info, unicode rendering
    ├── config.rs          # TOML config loading (~/.config/runits/config.toml)
    ├── grammar.pest       # pest grammar for expressions + compound-unit parsing
    ├── database/
    │   ├── CLAUDE.md      # module-specific instructions
    │   ├── mod.rs         # UnitDatabase: lookup, prefix stripping, fuzzy suggest, global singleton
    │   └── seed.rs        # seed_all(): ~63 builtin units + aliases
    ├── repl/
    │   ├── CLAUDE.md      # module-specific instructions
    │   ├── mod.rs         # REPL loop, input dispatch, ? help handlers, banner, info command
    │   └── helper.rs      # UnitsHelper: rustyline Completer/Hinter/Highlighter/Validator
    └── units/
        ├── CLAUDE.md      # module-specific instructions
        ├── mod.rs         # module re-exports
        ├── dimension.rs   # Dimension enum + DimensionMap + analysis_symbol/base_symbol
        ├── unit.rs        # Unit struct, ConversionKind, prefixable, Mul/Div, render_dimensions
        └── quantity.rs    # Quantity struct, conversion, format_value, arithmetic (try_add/try_sub/mul/div/pow_i32/neg)
```

---

## Development Environment

- **Language:** Rust (edition 2024)
- **Toolchain:** rustc 1.94.1, cargo 1.94.1 (as of 2026-03)
- **Dependencies (current):** `clap` (derive) + `clap_complete`, `pest` + `pest_derive`, `thiserror`, `owo-colors`, `rustyline`, `strsim`, `serde` (derive) + `toml`; dev: `assert_cmd`, `predicates`.

## Build & Development Commands

```bash
cargo check                          # compile check
cargo build                          # debug build
cargo build --release                # release build
cargo run                            # run demo
cargo run -- "10 ft" "m"             # run with args
cargo test                           # all tests (unit + doc + integration)
cargo test --doc                     # doc tests only
cargo doc --no-deps --open           # generate + open API docs (incl. roadmap chapter)
cargo doc --document-private-items   # include private items
cargo fmt --check                    # check formatting
cargo clippy -- -D warnings          # lint, warnings-as-errors
```

## Dependencies by Phase

| Crate | Phase | Purpose | Status |
|---|---|---|---|
| `clap` (derive) | 2 | CLI argument parsing | ✅ |
| `thiserror` | 2 | error derive macros | ✅ |
| `pest` + `pest_derive` | 2 | parser generator / grammar | ✅ |
| `assert_cmd` + `predicates` | 2 (dev) | CLI integration tests | ✅ |
| `rustyline` | 4 | interactive REPL + hinter/highlighter/completer | ✅ |
| `strsim` | 4 | fuzzy unit-name suggestions | ✅ |
| `owo-colors` | 4 | dimension-based colored output (Flexoki-inspired) | ✅ |
| `clap_complete` | 4 | shell completions (bash/zsh/fish) | ✅ |
| `serde` + `toml` | 4 | TOML config file | ✅ |

Full feature catalog with phase affinity lives in `docs/roadmap.md`.

## Code Style & Conventions

- Standard formatting: `cargo fmt`
- Clippy-clean: `cargo clippy -- -D warnings`
- Prefer `Result` over panics for recoverable failures
- Leverage the type system for dimensional safety (the core thesis of the project)
- Factory methods (e.g., `Unit::meter()`) for common constructions
- Doc-test every public API example
- Commit messages: short imperative subject (~50 char), no co-author lines
- **FUTURE markers:** `FUTURE(tag): description` in comments. Tag is kebab-case, groups related call-sites (e.g. `alias-types`). Both tag and description are required. Grep with `rg 'FUTURE\('` to list all

## Testing Strategy & TDD Workflow

This project follows **red/green/refactor TDD** — every new behavior starts with a failing test. No exceptions.

### The mandatory cycle

1. **Red** — Write the test first. Run `cargo test` and confirm it fails with a meaningful assertion error (not a build error). If the function doesn't exist yet, add a minimal stub (`todo!()` or `unimplemented!()`) so the test can compile and fail at runtime.
2. **Green** — Write the minimum implementation to make the test pass. `cargo test` must be fully green before moving on.
3. **Refactor** — Clean up without breaking tests. Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` in sequence before committing.

### Non-negotiable rules

- **Tests before code.** Implementation code for a new behavior must not be written until a failing test for that behavior exists and has been observed to fail.
- **Tests must fail for the right reason.** A test that fails only because it doesn't compile is not in the red phase yet — stub the API first.
- **One test group at a time.** Write one focused test (or one coherent group), confirm red, implement, confirm green, then add the next. Batching tests with implementation produces tests that were never red.
- **The test count in `docs/roadmap.md` Status grows monotonically.** Every feature adds tests; the count is updated after the green phase.

### Where tests live

- **Unit tests** — `#[cfg(test)] mod tests { ... }` at the bottom of the relevant source file (alongside the implementation it tests)
- **Doc tests** — in `///` doc comments on every public API; they double as API examples
- **Integration tests** — `tests/cli_tests.rs` for CLI behavior (`assert_cmd` + `predicates`)
- **Property-based tests** — optional; see deferred track in roadmap

### Test naming conventions

- Unit tests: `test_<behavior>` or `<subject>_<condition>_<expected_outcome>` (e.g., `add_incompatible_dims_returns_error`)
- Integration tests: describe the full user-visible behavior (`converts_miles_to_km`, `unknown_unit_suggests_correction`)

---

## Project Instructions

- **Default mode: Claude Code implements the majority of the code**, always in TDD order: failing test → minimum implementation → refactor. Keep changes focused, use liberal comments to explain *why* (not *what*), and ship working increments. This is a 70/30 polished-tool/learning project per the roadmap — do not turn every implementation into a tutorial.
- **Hand off when the learning is worth it.** When you hit a genuinely interesting Rust concept — trait objects, lifetimes, advanced pattern matching, clever ownership design, unsafe, macros, async internals, custom derive, etc. — pause, offer a concise hint with code comments marking the spot, and **ask the user if they want to take the lead on that specific piece**. The user decides; don't guess. Good candidates for hand-off: first encounter with a concept in the project, a design decision with multiple valid approaches, idiomatic-Rust "aha" moments.
- **Expect the learning bar to rise.** Phases 1–4 covered structs, enums, HashMap, Result, parser-generator macros (`pest`), grammar files, error-type ergonomics (`thiserror`), affine conversions, compound-unit algebra, REPL lifetimes (`rustyline`), `Helper` trait composition (Completer/Hinter/Highlighter/Validator), static initialization, config deserialization (`serde` + `toml`), and dimension-based theming (`owo-colors`). Upcoming phases introduce: trait objects (`Box<dyn>`), expression evaluators, file-format parsers, possibly `unsafe` or custom derive. Flag these moments explicitly when they appear — the user may want to slow down and write them personally even if earlier decisions were "Claude implements."
- **Code review mode:** when the user asks you to review their code, check understanding by asking targeted questions about *why* particular choices were made, not just whether they compile.
- **Whenever a change touches or implies a change to the project's plan, update `docs/roadmap.md` first** (see the "Working with the Roadmap" section above).
