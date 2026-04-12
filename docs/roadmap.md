# RUnits Roadmap

Single source of truth for **status**, **next phases**, and the **feature catalog**.

**Navigation:** [Status](#status) · [Phase 2](#phase-2--functional-cli) · [Phase 3](#phase-3--rich-conversions) · [Phase 4](#phase-4--interactive-experience) · [Phase 5a](#phase-5a--expressions--discovery) · [Phase 5b](#phase-5b--extensibility--ux) · [Phase 5c](#phase-5c--gnu-units-database-migration) · [Deferred Track](#deferred--optional-track) · [Extras Catalog](#extras-catalog) · [Design Principles](#design-principles)

---

## Status

| Phase | Status | Notes |
|---|---|---|
| 1 — Core Data Structures | ✅ Complete | `Dimension`, `Unit`, `Quantity` with full dimensional analysis; 7 SI base units + angle/information; `Mul`/`Div` traits for compound units |
| 1.5 — Documentation Foundation | ✅ Complete | Rustdoc on all public APIs; doc-tests; GitHub Actions → GitHub Pages |
| 2 — Functional CLI | ✅ Complete | clap, pest parser, UnitDatabase (~80 aliases), thiserror; 6-sig-fig adaptive output formatter; bare unit names accepted |
| 3 — Rich Conversions | ✅ Complete | ConversionKind enum (affine); temperature (C/F/K/Ra/Ré); SI prefixes (24) + binary (6); compound-unit grammar (`kg*m/s^2`); `--precision`/`--scientific`/`--to-base` flags; annotations registry; ~63 units + force/pressure/energy/power/historical/cooking/astronomical/radioactivity |
| 4 — Interactive Experience | ✅ Complete | REPL (rustyline), dimension-based color theme (Flexoki-inspired), Fish-style hinter + syntax highlighter, dimension-aware tab-completion, `?` help with SI base/factor/prefix, `info` command, long/short/off banner, fuzzy suggestions (strsim), `--json`/`--pretty`/`--batch`, TOML config (`~/.config/runits/config.toml`), shell completions, `Unit.prefixable`, Theme carries color flag |
| 4.5 — Codebase Reorganization | ✅ Complete | Split database/ (seed extraction), theme.rs (from format), repl/ (helper extraction); removed roadmap from rustdoc; CLAUDE.md hierarchy |
| 5a — Expressions & Discovery | ⏳ Active | Done: physical constants (15 CODATA values), conformable unit discovery (`list`/`search` commands + CLI subcommand), `--explain` flag + REPL `explain` command (unified linear/affine layout with standout calculation), unified REPL command dispatch with prefix-matching `list` subcommands, expression foundation (`Expr` AST + `EvalContext` tree walker + `MathFn` enum-dispatch registry with 8 initial functions: sqrt/sqr/abs/sin/cos/tan/ln/exp), math expressions in input (`3*4 meter`, `2^10 byte`), unit arithmetic with dimensional checking (`5 m + 3 ft`), previous-result `_` in REPL. Remaining: colored dimensional error messages |
| 5b — Extensibility & UX | ⏳ Planned | User-defined units, unit-list decomposition, scale chaining, reverse lookup |
| 5c — Database Expansion & Definition Format | ⏳ Planned | Numbat-inspired definition format, tiered loading (linear → recursive → nonlinear), domain modules (periodic table, astronomy), `--db` flag |

**Test suite (latest):** 213 unit tests + 10 doc tests + 39 integration tests = 262 total, all passing. Dependencies: clap, clap_complete, pest, pest_derive, thiserror, owo-colors, rustyline, strsim, serde, toml (dev: assert_cmd, predicates). Clean clippy, clean fmt.

For a detailed change history, see `git log`. For a feature-by-feature comparison with GNU Units, see [`docs/gnu-units-parity.md`](gnu-units-parity.md).

---

## Phase 2 — Functional CLI

**Goal:** `runits "10 ft" "m"` produces correct output for ~30–50 common units.

**Scope**
- `clap` (derive API) for argument parsing
- `pest` grammar for quantity parsing **from day one** (no hardcoded-parser detour)
- `thiserror` to migrate from the hand-rolled `ConversionError` to a unified error enum
- `UnitDatabase` struct holding the units + aliases (m, meter, metres, meters, …)
- Integration tests with `assert_cmd` + `predicates`
- Helpful CLI errors: unknown unit, incompatible dimensions, parse failure

**Rationale for skipping the hardcoded-parser detour.** The original plan wrote a match-based parser in Phase 2, then deleted it when `pest` arrived in Phase 3. Given the 70% polished-tool focus, that detour costs effort without leaving a lasting artifact. `pest`'s grammar for `number unit` is ~10 lines; jumping straight there also enforces a cleaner separation (parse → lookup → convert).

**Deliverable:** `cargo install --path .` yields a working CLI for everyday conversions.

**New files**
- `src/cli.rs` — clap-derived `Cli` struct
- `src/parser.rs` — `pest` parser + parse-tree walker
- `src/database.rs` — `UnitDatabase` with alias lookup
- `src/error.rs` — unified error enum via `thiserror`
- `src/grammar.pest` — grammar file
- `tests/cli_tests.rs` — `assert_cmd` integration tests

**Learning insight.** `thiserror` generates `Display`, `Error`, and `From` impls from attribute macros — you write the variant, it writes the boilerplate. Compare side-by-side with the current hand-rolled `ConversionError` in `quantity.rs` to see the delta.

---

## Phase 3 — Rich Conversions

**Goal:** Handle the real-world inputs people actually type.

**Scope**
- **SI prefixes** (yotta Y → yocto y, 24 levels) applied to any unit: `kmeter`, `µsecond`, `Gbyte`
- **Binary prefixes** (Ki, Mi, Gi, Ti, Pi, Ei) for information units
- **Non-linear conversions** for temperature (Celsius, Fahrenheit, Kelvin, Rankine, Réaumur)
- **Compound-unit parsing** (`kg*m/s^2`, `5 kg/m^3`) via extended `pest` grammar
- **Output formatting** — precision control, scientific notation, significant figures
- **Result representation policy** — default keeps named derived units in compact compound form (Numbat-style: Coulomb's constant renders as `8.99e9 m/F`); opt-in `--to-base` expands every named unit to the 7 SI primitives (GNU units-style: same value becomes `8.99e9 kg·m³·A⁻²·s⁻⁴`). Default optimizes for readability and "paste into a report"; opt-in optimizes for dimensional analysis and teaching. The tool is a modern GNU Units, not a Numbat competitor — so both ship.

**Design decision — affine conversions.** The current `Unit` uses a single `conversion_factor: f64`. Temperature requires `scale + offset` (e.g., °F = (K − 273.15) × 9/5 + 32). Two viable designs:

1. **Enum variant**: `ConversionKind::Linear(f64)` vs `ConversionKind::Affine { scale: f64, offset: f64 }` on `Unit`.
2. **Transform function**: `to_base: fn(f64) -> f64` + `from_base: fn(f64) -> f64` on `Unit` (more general, harder to compose).

Pick option 1 for Phase 3 — it composes with existing multiplicative `Mul`/`Div` when kept separate from compound units (temperature differences *are* linear; only absolute temperature is affine). Document this carefully in `docs/design-decisions.md`.

**Deliverable:** `runits "98.6 F" "C"` and `runits "100 kmeter/hour" "mile/hour"` both work.

---

## Phase 4 — Interactive Experience

**Goal:** REPL and CLI UX that's actively pleasant.

**Scope**
- `rustyline` REPL with persistent history (`~/.config/runits/history`)
- **Fuzzy suggestions** on unknown units via `strsim` (Levenshtein / Jaro-Winkler)
- **Colored output** via `owo-colors` (respects `NO_COLOR` env var)
- **Unicode unit rendering** — middle-dot for multiplication (`kg·m`), superscript digits for exponents (`s⁻²`), proper minus signs. CLI output is piped/copy-pasted, so plain ASCII by default; REPL and `--pretty` opt-in to Unicode.
- **TOML config** at `~/.config/runits/config.toml` — default precision, color on/off, preferred output format
- **Shell completions** via `clap_complete` (bash/zsh/fish)
- **Output modes**: plain (default), verbose (show conversion chain), JSON (scriptable)
- **Batch mode**: one query per line from stdin
- **Physical-quantity annotations in REPL** — dimension-name annotations like "Velocity" and "Force" (Phase 3 registry) are shown only in interactive REPL mode, following Numbat's convention. CLI output stays clean for piping. Annotations may be colored/formatted when `owo-colors` is available.

**Deliverable:** `runits` (no args) opens a REPL; typos suggest corrections; config respected.

---

## Phase 5a — Expressions & Discovery

**Goal:** Let users compute, not just convert — and find units they don't know the name of.

**Scope**
- **Physical constants** database (c, G, h, ℏ, k_B, N_A, R, e, ε₀, µ₀, g) — `runits const c` prints `2.998e8 m/s`
- **Math expressions** in input (`runits "3*4 meter" "foot"`, `2^10 byte to kB`)
- **Unit arithmetic** (`5 meter + 3 foot` with dimensional checking)
- **Previous-result variable `_`** — stores the last conversion result, usable in subsequent expressions (like Python's REPL `_`)
- **`--explain` flag** — show step-by-step conversion chain with dimensional reasoning (`100 km/h → ×1000/3600 → 27.78 m/s`). Promoted from Extras Catalog.
- **Dimensional error messages** — colored, source-spanned errors for invalid expressions (`"cannot add Length + Time"` with the offending subexpression highlighted in its dimension color). Promoted from Deferred track — building an expression evaluator with bad errors would waste the opportunity.
- **Conformable unit discovery** — `search velocity` in REPL lists all known units of that dimension, grouped by physical quantity. `--list-units` CLI flag. Extends the existing `?` help system.

**Deliverable:** `runits` becomes a dimensional micro-calculator with rich error feedback.

**Expression foundation (shipped).** The parser now produces an `Expr` AST (`Number`/`Ident`/`Previous`/`BinOp`/`Neg`/`Pow`/`FuncCall`) and an `EvalContext`-driven tree walker evaluates it into a `Quantity`. Math functions live in a `MathFn` enum with exhaustive match dispatch — adding a new Numbat-style function (e.g. `log10`, `cbrt`, `atan2`) is one enum variant plus four match arms (`name`, `signature`, `apply`, `ALL`), and the compiler enforces that every site stays in sync. Dimensional checking at every binop delegates to `Quantity::try_add`/`try_sub`/`mul`/`div`/`pow_i32` inherent methods with a shared affine-rejection path. The REPL tracks `last_quantity` so `_` refers to the most recent successful eval. Errors carry structured context (both dim strings on mismatches, op context on affine rejection, name+suggestions on unknown identifiers/functions) so the follow-up colored-errors step only has to render, not re-plumb.

---

## Phase 5b — Extensibility & UX

**Goal:** User customization and practical output modes.

**Scope**
- **User-defined units** via `~/.config/runits/units.conf` (syntax: `furlong = 220 yard`)
- **User-defined dimension names** in the same config (syntax: `dimension Torque = Force × Length`) — extends the annotation registry at runtime. Pure HashMap entries, not type-system work.
- **Named variables in REPL** — `x = 5 ft`, then `x to m`. Extends the Phase 5a expression evaluator with a `HashMap<String, Quantity>` store.
- **Unit-list decomposition output** — `6.25 ft → 6 ft 3 in`, `5400 s → 1 h 30 min`. Ergonomic syntax: `runits "6.25 ft" "ft+in"` or `--decompose`. Promoted from Extras Catalog — practical, moderate effort.
- **Scale chaining input** — `10 ft 5 in` parsed as compound length
- **Reverse lookup** — given a dimensioned value, suggest matching units/constants (`runits --what "9.81 m/s^2"` → `gravity (g)`), ranked by closeness

**Deliverable:** Users can extend the database, get human-readable mixed output, and explore units by value.

---

## Phase 5c — Database Expansion & Definition Format

**Goal:** Grow the unit database from ~63 builtins to thousands, via a clean definition-file format inspired by Numbat's module system — then use it to absorb GNU Units' data and add domain modules like the periodic table.

This is the single deepest gap vs GNU Units (see `docs/gnu-units-parity.md`). Rather than parsing GNU's legacy format directly, RUnits defines its own declarative format and uses it to express both GNU-sourced data and new domain modules. The codebase stays clean; the data scales.

**Definition file format**

Numbat-inspired, declarative (not an interpreted language). Syntax TBD in detail, but the shape:

```
# Module inclusion (main file → sub-modules)
include "chemistry/elements"
include "extra/astronomy"

# Dimension declarations (extend the builtin set)
dimension Torque = Force * Length
dimension MolarEnthalpy = Energy / Amount

# Unit definitions — typed, with optional aliases
unit furlong: Length = 220 yard
unit solar_mass: Mass = 1.98847e30 kg      @aliases(M_sun)
unit jansky: SpectralFluxDensity = 1e-26 W / m^2 / Hz

# Constants
const c: Velocity = 299792458 m/s
const G = 6.67430e-11 m^3 / (kg * s^2)
```

Key design decisions vs Numbat: no `use` (we use `include` — flat, not a module tree with namespaces), no executable code, annotations via `@key(value)` rather than Numbat's decorator style. Vs GNU Units: no directives (`!locale`, `!set`, `!var`), no fraction syntax (`5|9`) — these are handled by config.toml and standard decimal notation respectively.

**Scope**

**Tier 1 — Builtin (already done):** Hand-seeded ~63 units + dynamic SI/binary prefixes. Zero I/O, instant startup.

**Tier 2 — Linear definitions:** Parse the definition format above for simple linear units. `include` support for modular files. Adds ~500–1,000 units from GNU-sourced data converted to our format. Skips unresolvable lines. Parses at startup or on first use.

**Tier 3 — Recursive resolution:** Resolve definition chains (`foot 12 inch` → look up `inch` → resolve to meters). Prefix definitions (`name- factor`). Cycle detection. ~2,000–3,000 units.

**Tier 4 — Nonlinear functions:** Function-defined units like temperature scales (`tempC(x) = x + 273.15`), piecewise linear tables (wire gauges, shoe sizes), and their inverses. Architecturally distinct — requires a mini expression evaluator and forward/inverse function dispatch.

**Domain modules**

- **Periodic table** (`chemistry/elements`) — all elements with symbol, name, atomic number, atomic mass (in Da), density, melting/boiling points. Inspired by [Numbat's elements module](https://github.com/sharkdp/numbat/blob/main/numbat/modules/chemistry/elements.nbt). Lookup by symbol or name: `element H` → hydrogen properties.
- **Astronomy** (`extra/astronomy`) — solar/planetary constants, distance scales (AU, ly, pc, kpc), luminosity units
- **Nuclear/atomic** — barn, amu, Planck units
- **Additional SI derived** — area (hectare, acre), concentrations (ppm, molarity)

**Infrastructure**
- Selection via `--db` flag (`--db builtin|standard|full`) or TOML config default
- REPL defaults to standard; one-shot CLI defaults to builtin for speed
- `--check` flag to validate the database (report unresolvable definitions, cycles)
- Logs skipped/unresolvable lines at `--debug` level
- Definition files ship with the binary (embedded via `include_str!` or loaded from `~/.config/runits/modules/`)

**Deliverable:** `runits --db full` loads thousands of units from modular definition files, including domain-specific modules and nonlinear function-defined conversions.

---

## Deferred / Optional Track

Architecturally interesting work with narrower user value — tackle when motivation strikes:

- **Currency conversion** with live exchange-rate API (e.g., exchangerate.host). Requires HTTP client (reqwest/ureq) + cache layer + rate staleness logic. Architecturally distinct from everything in Phase 5. Potential Phase 6.
- **Multiple unit systems** (CGS, Imperial, Natural). Great trait-object learning (`Box<dyn UnitSystem>`, strategy pattern), but the value for most users is narrow — compound units with prefixes already cover practical needs.
- **TUI mode** (`runits --tui`) via `ratatui` — a standalone full-screen interactive mode, separate from the REPL. Live dropdown fuzzy picker, side panel with unit info, dimension-colored suggestions. This is *not* a replacement for the REPL — it's an alternative interface. The REPL uses rustyline with progressively enhanced Fish-style completion (hinter, highlighter, dimension-aware tab); the TUI is a distinct full-screen experience with fzf-style filtering.
- **WASM target** with a small web playground.
- **Quality tooling**: criterion benchmarks, proptest round-trip tests, cargo-fuzz on the parser, cargo-dist release packaging, Homebrew tap.
- **Significant-figure-aware arithmetic** — track sig figs through conversions, warn on excess precision. Niche but loved by chemistry/physics students.
- **Constants with CODATA uncertainty** — physical constants with official uncertainties (`G = 6.67430(15)×10⁻¹¹ m³/(kg·s²)`). Could be a quality aspect of the Phase 5a constants implementation.

### Explicitly not planned

- **Integer fraction syntax** (`1|2 meter`) — confusing, collides with shell pipe expectations, `0.5 meter` serves the same need. GNU Units inherited this from its 1980s origin.
- **GNU Units directives** (`!locale`, `!set`, `!var`, `!message`, `!unitlist`) — file-format machinery for GNU's monolithic definition file. RUnits handles these concerns differently: locale via config.toml, decomposition via Phase 5b's `--decompose`, parser options via CLI flags.
- **Multivariate function units** (`windchill(temp, speed)`) — too specialized, minimal user value vs. implementation cost.

---

## Extras Catalog

Each item tagged with a **phase affinity** — when you're working on that phase, scan here for features to pull in. This is a menu, not a mandatory sequence.

### CLI UX Polish

| # | Feature | Phase |
|---|---|---|
| 1 | Colored output (`owo-colors`, respects `NO_COLOR`) | 4 |
| 2 | Shell completions (bash/zsh/fish via `clap_complete`) | 4 |
| 3 | TOML config file (`~/.config/runits/config.toml`) | 4 |
| 4 | Output formats: plain / verbose / JSON / CSV | 4 |
| 5 | `--verbose` / `--quiet` / `--debug` flags | 2 |
| 6 | Batch mode (stdin piping, one query per line) | 4 |
| 7 | `--explain` flag (show conversion chain + dimensions) | 5a |
| 8 | `--precision N` / `--scientific` output flags | 3 |
| 9 | Tab completion inside REPL (unit names) | 4 |
| 10 | Ctrl+R history search in REPL | 4 |
| 11 | Pretty errors with source spans (`miette` / `ariadne`) | 5a |
| 12 | TUI mode with `ratatui` (unit browser + converter) | Optional |
| 13 | `--dry-run` (parse & validate without computing) | 2 |
| 14 | Man-page generation (`clap_mangen`) | 4 |

### Advanced Conversions

| # | Feature | Phase |
|---|---|---|
| 1 | Temperature scales: C / F / K / Rankine / Réaumur | 3 |
| 2 | SI prefixes (yotta → yocto, 24 levels) | 3 |
| 3 | Binary prefixes (Ki → Ei, for info units) | 3 |
| 4 | Unit arithmetic: `5m + 3ft`, `100kg - 200g` | 5a |
| 5 | Scale chaining: `6ft 5in`, `1yr 3mo 2wk` | 5b |
| 6 | Math expressions: `3*4.5 + 2 meter` | 5a |
| 7 | Compound name simplification (`meter*meter` → `meter^2`, `m·s⁻¹·s⁻¹` → `m·s⁻²`) — touches Mul/Div core | 5a |
| 8 | Reverse lookup (`what is 9.81 m/s²?` → gravity) | 5b |
| 9 | Significant-figure-aware arithmetic | 3 |
| 10 | Angles: rad/deg/grad/turn/arcmin/arcsec | 2 |
| 11 | Logarithmic scales: dB, neper, phon, pH, Richter | Optional |
| 12 | E=mc² energy↔mass equivalence | Deferred |
| 13 | Frequency↔wavelength via c (λν=c) | 5 |
| 14 | Unit-list decomposition (`2.5 ft` → `2 ft 6 in`) | 5b |
| 15 | Named physical-quantity annotations (e.g. Velocity, Acceleration) via a dimension-signature → name registry (display side only, not type-system work). Registry built in Phase 3; display is REPL-only (Phase 4), following Numbat's convention — CLI output stays pipe-friendly. | 3/4 |

### Database & Data Enrichment

| # | Feature | Phase |
|---|---|---|
| 1 | GNU `definitions.units` incremental parser with tiered loading (builtin/standard/full) | 5c |
| 2 | Unit aliases (m, meter, meters, metres) | 2 |
| 3 | Historical (cubit, league, furlong, stone, rod, chain, perch) | 3 |
| 4 | Cooking (cup, tbsp, tsp, fl oz, gill, drachm) | 3 |
| 5 | Astronomical (AU, ly, pc, kpc, Mpc, solar mass/radius) | 3 |
| 6 | Nuclear/atomic (barn, eV, amu, Planck units) | 5c |
| 7 | Physical constants (c, G, h, ℏ, k_B, N_A, R, e, ε₀, µ₀, g) | 5a |
| 8 | Regional variants (US/Imperial gallon, troy/avoirdupois oz, long/short ton) | 3 |
| 9 | Computer/digital (Hz, RPM, FPS, DPI, PPI) | 3 |
| 10 | Sound (dB, dBm, phon, sone) | Optional |
| 11 | Photometry (lux, lumen, candela, nit, stilb) | 2 |
| 12 | Seismology (Richter, Mercalli, MMS) | Optional |
| 13 | Pressure (Pa, bar, psi, atm, torr, mmHg, inHg) | 3 |
| 14 | Radioactivity (becquerel, curie, sievert, gray, rem, rad) | 3 |
| 15 | Concentrations (molar, molal, ppm, ppb, %w/w, %v/v) | 3 |
| 16 | Area (are, hectare, acre, barn, square foot/inch/mile) | 5c |

---

## Design Principles

Guiding trade-offs for every decision in this project:

1. **Correctness over convenience.** Dimensional safety comes first — the type system should make invalid conversions impossible at compile time wherever feasible, and loud failures at runtime otherwise.
2. **Pleasant UX by default.** Colored output, helpful errors with suggestions, a REPL that feels nice. These aren't polish — they're what makes the tool worth reaching for.
3. **Self-contained.** No mandatory external services, no required config files, no required network. Optional features (currency rates) are clearly opt-in.
4. **Readable over clever.** Borrow liberally, clone early, optimize later. The code should read well for someone learning Rust.
5. **Tests as documentation.** Doc tests double as API examples; integration tests document CLI behavior; property tests document invariants.
