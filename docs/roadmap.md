# RUnits Roadmap

Single source of truth for **status**, **next phases**, and the **feature catalog**.
This file is embedded into the rustdoc site and rendered as a chapter there.

**Navigation:** [Status](#status) · [Phase 2](#phase-2--functional-cli) · [Phase 3](#phase-3--rich-conversions) · [Phase 4](#phase-4--interactive-experience) · [Phase 5](#phase-5--extensibility--power-user-features) · [Deferred Track](#deferred--optional-track) · [Extras Catalog](#extras-catalog) · [Design Principles](#design-principles)

---

## Status

| Phase | Status | Notes |
|---|---|---|
| 1 — Core Data Structures | ✅ Complete | `Dimension`, `Unit`, `Quantity` with full dimensional analysis; 7 SI base units + angle/information; `Mul`/`Div` traits for compound units |
| 1.5 — Documentation Foundation | ✅ Complete | Rustdoc on all public APIs; doc-tests; GitHub Actions → GitHub Pages |
| 2 — Functional CLI | ✅ Complete | clap, pest parser, UnitDatabase (~80 aliases), thiserror; 6-sig-fig adaptive output formatter; bare unit names accepted |
| 3 — Rich Conversions | ✅ Complete | ConversionKind enum (affine); temperature (C/F/K/Ra/Ré); SI prefixes (24) + binary (6); compound-unit grammar (`kg*m/s^2`); `--precision`/`--scientific`/`--to-base` flags; annotations registry; ~63 units + force/pressure/energy/power/historical/cooking/astronomical/radioactivity |
| 4 — Interactive Experience | ⏳ Active | REPL (rustyline), dimension-based color theme (Flexoki-inspired), Fish-style hinter + syntax highlighter, dimension-aware tab-completion, `?` help with analysis/base-unit/factor, fuzzy suggestions (strsim), `--json`/`--pretty`/`--batch`, TOML config, shell completions, `Unit.prefixable` |
| 5 — Extensibility | ⏳ Planned | Custom units, constants, expressions, GNU units parser |

**Test suite (latest):** 112 unit tests + 9 doc tests + 19 integration tests = 140 total, all passing. Dependencies: clap, clap_complete, pest, pest_derive, thiserror, owo-colors, rustyline, strsim, serde, toml (dev: assert_cmd, predicates). Clean clippy, clean fmt.

For a detailed change history, see `git log`.

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

## Phase 5 — Extensibility & Power-User Features

**Goal:** Let users customize and compute, not just convert.

**Scope**
- **User-defined units** via `~/.config/runits/units.conf` (syntax: `furlong = 220 yard`)
- **User-defined dimension names** in the same config (syntax: `dimension Torque = Force × Length`) — extends the annotation registry (Phase 3 feature) at runtime. Pure HashMap entries, not type-system work.
- **Physical constants** database (c, G, h, ℏ, k_B, N_A, R, e, ε₀, µ₀, g) — `runits const c` prints `2.998e8 m/s`
- **Math expressions** in input (`runits "3*4 meter" "foot"`)
- **Unit arithmetic** (`5 meter + 3 foot` with dimensional checking)
- **Scale chaining** (`10 ft 5 in` parsed as compound length)
- **Reverse lookup** — given a dimensioned value, suggest matching units/constants (`runits --what "9.81 m/s^2"` → `gravity (g)`)
- **GNU `definitions.units` incremental parser** with **tiered database loading**:
  - **Tier 1 — Builtin (default):** Hand-seeded ~63 units + dynamic SI/binary prefixes. Zero I/O, instant startup. Sufficient for everyday conversions.
  - **Tier 2 — Standard:** Tier 1 + simple `name definition` lines from GNU `definitions.units` where the definition resolves to known units. Adds ~500–1000 units (pressure, cooking, historical, etc.). Parses on first use or at startup.
  - **Tier 3 — Full:** Tier 2 + recursive definition resolution (`foot 12 inch` → look up `inch`), fraction syntax (`5|9`), `!include` for personal units files. ~2000–3000 units.
  - Selection via `--db` flag (`--db builtin|standard|full`) or TOML config default. REPL defaults to standard; one-shot CLI defaults to builtin for speed.
  - Skips directives (`!locale`, `!set`, `!var`), function definitions (`tempC(x)`), and base markers (`m !`). Logs skipped lines at `--debug` level.

**Deliverable:** `runits` becomes a dimensional micro-calculator.

---

## Deferred / Optional Track

Architecturally interesting work with narrower user value — tackle when motivation strikes:

- **Multiple unit systems** (CGS, Imperial, Natural). Great trait-object learning (`Box<dyn UnitSystem>`, strategy pattern), but the value for most users is narrow — compound units with prefixes already cover practical needs.
- **Currency conversion** with live exchange-rate API (e.g., exchangerate.host). Requires HTTP client + cache layer.
- **TUI mode** (`runits --tui`) via `ratatui` — a standalone full-screen interactive mode, separate from the REPL. Live dropdown fuzzy picker, side panel with unit info, dimension-colored suggestions. This is *not* a replacement for the REPL — it's an alternative interface. The REPL uses rustyline with progressively enhanced Fish-style completion (hinter, highlighter, dimension-aware tab); the TUI is a distinct full-screen experience with fzf-style filtering.
- **WASM target** with a small web playground.
- **Error message polish** — current error messages are functional but basic. Improvements: dimension-colored dimension names in error output, suggested conversions ("did you mean to convert to a Force unit?"), `miette`/`ariadne` source spans for parse errors. Could be a Phase 6 "general polish" pass.
- **Quality tooling**: criterion benchmarks, proptest round-trip tests, cargo-fuzz on the parser, cargo-dist release packaging, Homebrew tap.

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
| 7 | `--explain` flag (show conversion chain + dimensions) | 3 |
| 8 | `--precision N` / `--scientific` output flags | 3 |
| 9 | Tab completion inside REPL (unit names) | 4 |
| 10 | Ctrl+R history search in REPL | 4 |
| 11 | Pretty errors with source spans (`miette` / `ariadne`) | 2+ |
| 12 | TUI mode with `ratatui` (unit browser + converter) | Optional |
| 13 | `--dry-run` (parse & validate without computing) | 2 |
| 14 | Man-page generation (`clap_mangen`) | 4 |

### Advanced Conversions

| # | Feature | Phase |
|---|---|---|
| 1 | Temperature scales: C / F / K / Rankine / Réaumur | 3 |
| 2 | SI prefixes (yotta → yocto, 24 levels) | 3 |
| 3 | Binary prefixes (Ki → Ei, for info units) | 3 |
| 4 | Unit arithmetic: `5m + 3ft`, `100kg - 200g` | 5 |
| 5 | Scale chaining: `6ft 5in`, `1yr 3mo 2wk` | 5 |
| 6 | Math expressions: `3*4.5 + 2 meter` | 5 |
| 7 | Compound name simplification (`meter*meter` → `meter^2`, `m·s⁻¹·s⁻¹` → `m·s⁻²`) — touches Mul/Div core | 5 |
| 8 | Reverse lookup (`what is 9.81 m/s²?` → gravity) | 5 |
| 9 | Significant-figure-aware arithmetic | 3 |
| 10 | Angles: rad/deg/grad/turn/arcmin/arcsec | 2 |
| 11 | Logarithmic scales: dB, neper, phon, pH, Richter | Optional |
| 12 | E=mc² energy↔mass equivalence | Deferred |
| 13 | Frequency↔wavelength via c (λν=c) | 5 |
| 14 | Fractional display (`2.5 ft` → `2 ft 6 in`) | 3 |
| 15 | Named physical-quantity annotations (e.g. Velocity, Acceleration) via a dimension-signature → name registry (display side only, not type-system work). Registry built in Phase 3; display is REPL-only (Phase 4), following Numbat's convention — CLI output stays pipe-friendly. | 3/4 |

### Database & Data Enrichment

| # | Feature | Phase |
|---|---|---|
| 1 | GNU `definitions.units` incremental parser with tiered loading (builtin/standard/full) | 5 |
| 2 | Unit aliases (m, meter, meters, metres) | 2 |
| 3 | Historical (cubit, league, furlong, stone, rod, chain, perch) | 3 |
| 4 | Cooking (cup, tbsp, tsp, fl oz, gill, drachm) | 3 |
| 5 | Astronomical (AU, ly, pc, kpc, Mpc, solar mass/radius) | 3 |
| 6 | Nuclear/atomic (barn, eV, amu, Planck units) | 5 |
| 7 | Physical constants (c, G, h, ℏ, k_B, N_A, R, e, ε₀, µ₀, g) | 5 |
| 8 | Regional variants (US/Imperial gallon, troy/avoirdupois oz, long/short ton) | 3 |
| 9 | Computer/digital (Hz, RPM, FPS, DPI, PPI) | 3 |
| 10 | Sound (dB, dBm, phon, sone) | Optional |
| 11 | Photometry (lux, lumen, candela, nit, stilb) | 2 |
| 12 | Seismology (Richter, Mercalli, MMS) | Optional |
| 13 | Pressure (Pa, bar, psi, atm, torr, mmHg, inHg) | 3 |
| 14 | Radioactivity (becquerel, curie, sievert, gray, rem, rad) | 3 |
| 15 | Concentrations (molar, molal, ppm, ppb, %w/w, %v/v) | 3 |
| 16 | Area (are, hectare, acre, barn, square foot/inch/mile) | 5 |

---

## Design Principles

Guiding trade-offs for every decision in this project:

1. **Correctness over convenience.** Dimensional safety comes first — the type system should make invalid conversions impossible at compile time wherever feasible, and loud failures at runtime otherwise.
2. **Pleasant UX by default.** Colored output, helpful errors with suggestions, a REPL that feels nice. These aren't polish — they're what makes the tool worth reaching for.
3. **Self-contained.** No mandatory external services, no required config files, no required network. Optional features (currency rates) are clearly opt-in.
4. **Readable over clever.** Borrow liberally, clone early, optimize later. The code should read well for someone learning Rust.
5. **Tests as documentation.** Doc tests double as API examples; integration tests document CLI behavior; property tests document invariants.
