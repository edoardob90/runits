# GNU Units Parity Analysis

Reference document comparing RUnits against [GNU Units 2.27](https://www.gnu.org/software/units/manual/units.html).
Purpose: planning aid — not a promise to implement everything, but an honest map of the gap.

**Last updated:** 2026-04-08 (post Phase 4.5)

---

## What RUnits Already Covers

| Area | Status |
|---|---|
| Linear/multiplicative conversions | ✅ |
| SI prefixes (24 levels) + binary prefixes | ✅ |
| Temperature — affine (C / F / K / Rankine / Réaumur) | ✅ |
| Compound unit parsing (`kg*m/s^2`, `km/hr`, `m^3`) | ✅ |
| REPL with history, tab completion, hinter, highlighter | ✅ |
| Batch / pipe mode, JSON output | ✅ |
| Output flags (`--precision`, `--scientific`, `--to-base`) | ✅ |
| Fuzzy unit suggestions on typos | ✅ |
| Shell completions (bash/zsh/fish) + TOML config | ✅ |
| `info` command, `?` help, dimension-aware annotations | ✅ |

RUnits also has things GNU Units **does not**: dimension-colored output
(Flexoki-inspired), physical-quantity annotations in REPL, `--to-base` SI
primitive expansion, JSON output.

---

## Gap Table

### Expression Syntax

| Feature | GNU Units | RUnits |
|---|---|---|
| `*` `/` `^` compound units | ✅ | ✅ |
| Addition/subtraction of conformable units (`5m + 3ft`) | ✅ | ⏳ Phase 5 |
| Math functions (`sin`, `cos`, `sqrt`, `exp`, `ln`, `floor`, `erf`, `Gamma`, …) | ✅ (20+) | ⏳ Phase 5 partial |
| Integer fraction syntax (`1\|2 meter`) | ✅ | ❌ |
| Previous-result variable `_` in REPL | ✅ | ❌ |
| Runtime user variables (`_x = 2 ft`) | ✅ | ❌ |
| Spelled-out numbers (`seventeen`) | ✅ | ❌ |
| Parentheses in expressions | ✅ | ✅ (in compound units) |

### Conversion Types

| Feature | GNU Units | RUnits |
|---|---|---|
| Linear conversions | ✅ | ✅ |
| Affine (temperature absolute) | ✅ | ✅ |
| Temperature delta vs absolute (`degF` vs `tempF(x)`) | ✅ distinct | ✅ partial |
| Reciprocal auto-conversion (ohm ↔ siemens) | ✅ (`--strict` to suppress) | ❌ |
| Nonlinear function units (domain/range, forward/inverse) | ✅ | ❌ |
| Piecewise linear units (wire gauges, ring sizes, shoe sizes) | ✅ | ❌ |
| Multivariate functions (`windchill(temp, speed)`) | ✅ | ❌ |
| Inverse nonlinear (`~wiregauge(0.09 in)`) | ✅ | ❌ |
| Unit-list decomposition (`ft;in;1\|8 in`, `h;min;s`) | ✅ | ❌ |
| Conformable unit listing (`?` at "You want:") | ✅ | ❌ |

### Definition File Format

| Feature | GNU Units | RUnits |
|---|---|---|
| Simple linear: `name definition` | ✅ | ⏳ Phase 5 |
| Prefix definition: `name- factor` | ✅ | ❌ |
| `!include` for personal files | ✅ | ⏳ Phase 5 |
| Nonlinear function definitions | ✅ | ❌ |
| Piecewise linear table definitions | ✅ | ❌ |
| Multivariate function definitions | ✅ | ❌ |
| Directives (`!locale`, `!var`, `!set`, `!message`, `!unitlist`) | ✅ | ❌ |
| Unicode operator synonyms in files | ✅ | ❌ |
| Redefine-without-warning (`+name`) | ✅ | ❌ |

### Unit Database

| Area | GNU Units (~3,000 units) | RUnits (~80 builtins) |
|---|---|---|
| SI + derived (N, Pa, J, W, C, V, Ω, …) | ✅ | ✅ |
| Historical / traditional (cubit, furlong, stone, …) | ✅ | ✅ partial |
| Cooking (cup, tbsp, tsp, gill, …) | ✅ | ✅ partial |
| Astronomical (AU, ly, pc, solar mass, …) | ✅ | ✅ partial |
| Radioactivity (Bq, Ci, Sv, Gy, rem, …) | ✅ | ✅ partial |
| Physical constants (c, G, h, k_B, N_A, …) | ✅ | ⏳ Phase 5 |
| Atomic masses (all elements + isotopes) | ✅ (`elements.units`) | ❌ |
| Currency (live rates, FloatRates) | ✅ (external `units_cur` script) | ⏳ Deferred |
| US CPI / inflation functions | ✅ (`cpi.units`) | ❌ |
| Wire / pipe / screw gauges | ✅ (piecewise linear) | ❌ |
| CGS unit systems (Gaussian, ESU, EMU, HLU) | ✅ | ❌ Deferred |
| Natural / Planck / Hartree units | ✅ | ❌ |
| Ingredient densities (flour, sugar, …) | ✅ | ❌ |

### CLI Flags

| Flag | GNU Units | RUnits |
|---|---|---|
| `--precision N` / `--scientific` | ✅ (`-d N`, `-e`) | ✅ |
| `--to-base` (SI primitive expansion) | ❌ | ✅ (unique) |
| JSON / pretty output | ❌ | ✅ (unique) |
| Batch mode | ✅ | ✅ |
| `--conformable` (non-interactive) | ✅ | ❌ |
| `--list-units` (dump all, pipeable) | ✅ | ❌ |
| `--terse` / `--compact` / `--one-line` | ✅ | ❌ |
| `--strict` (suppress reciprocal) | ✅ | ❌ |
| `--round` (round list tail) | ✅ | ❌ |
| `--log FILE` (session log) | ✅ | ❌ |
| `--units SYSTEM` (CGS, natural, …) | ✅ | ❌ |
| `--locale` override | ✅ | ❌ |
| `--check` (validate database) | ✅ | ❌ |
| `--output-format` (printf-style) | ✅ | ❌ |

### Interactive / REPL Features

| Feature | GNU Units | RUnits |
|---|---|---|
| REPL with readline + history | ✅ | ✅ |
| Tab completion of unit names | ✅ | ✅ |
| Hinter / syntax highlighter | ❌ | ✅ (unique) |
| Dimension-colored output | ❌ | ✅ (unique) |
| Physical-quantity annotations | ❌ | ✅ (unique) |
| `?` → conformable unit listing | ✅ | ❌ (? = help in RUnits) |
| `search TEXT` — substring unit search | ✅ | ❌ |
| `help UNIT` — opens pager at definition | ✅ | ❌ |
| `set OPTION=VALUE` mid-session | ✅ | ❌ |
| Session logging | ✅ | ❌ |
| Locale-aware unit selection | ✅ | ❌ |
| Environment variables (`MYUNITSFILE`, `UNITS_ENGLISH`) | ✅ | ❌ |

---

## Parity Estimate

| Category | Parity |
|---|---|
| Common everyday conversions | ~70% |
| Expression syntax | ~40% |
| Definition file format | ~15% (linear only, Phase 5) |
| CLI flags | ~30% |
| Unit database breadth | ~5–10% pre-Phase 5 tiered loader |
| Interactive REPL features | ~50% |
| **Overall feature parity** | **~25–30%** |

The single deepest gap is the **definition file format** — GNU Units' nonlinear
functions, piecewise tables, and directives are what enable its ~3,000-unit
database. Phase 5's tiered loader closes the *linear* subset but the nonlinear
half is architecturally distinct and not yet on the roadmap.

---

## Standout Opportunities

Features that would make RUnits *better* than GNU Units in ways users actually
notice — ranked by value / differentiation / feasibility. See also the Extras
Catalog in `roadmap.md`.

### Tier 1 — High value, clearly worth doing

**1. Rich expression evaluator with dimensional error messages**
Phase 5 already plans math expressions, but the differentiator is the *quality*
of errors. GNU Units' error output is terse and monochrome. RUnits could give
colored, source-spanned errors: `"cannot add Length + Time"` with the offending
subexpression highlighted in its dimension color. This is the single most
impactful thing — it directly serves the "learning tool" use case that GNU
Units ignores.

**2. Unit-list decomposition output**
Convert to a human-readable mixed format: `6.25 ft` → `6 ft 3 in`, `5400 s` →
`1 h 30 min`, `1.5 yr` → `1 yr 6 mo`. GNU Units has this but it requires
knowing the `ft;in` syntax and defining `!unitlist` aliases. RUnits could
expose it ergonomically: `runits "6.25 ft" "ft+in"` or `--decompose`. Very
practical; low implementation complexity vs. impact.

**3. Conformable unit discovery**
`runits --what LENGTH` or `search velocity` in REPL lists all known units of
that dimension. GNU Units has `?` at the prompt and `--conformable`, but
it dumps a raw alphabetical list. RUnits could group by physical quantity
(Velocity: m/s, km/h, knot, mph, mach…), colored by dimension. Genuinely
useful for exploration and learning.

**4. Built-in currency with auto-fetch and cache**
GNU Units requires running a separate `units_cur` Python script to update
`currency.units`, then re-loading. RUnits could fetch on first use (or
`runits --update-rates`), cache to `~/.cache/runits/rates.json` with a
timestamp, and fall back to cached rates when offline. No external script, no
manual step. This is already in the Deferred track — worth pulling up.

### Tier 2 — Good value, moderate effort

**5. `--explain` flag (conversion chain)**
Show the step-by-step dimensional chain: `100 km/h → (×1000/3600) → 27.78
m/s`. Already in the Extras Catalog. Especially valuable for compound units
where the algebra is non-obvious. GNU Units doesn't have this.

**6. Reverse lookup with ranking**
`runits --what "9.81 m/s²"` currently planned for Phase 5. The differentiator:
rank results by how close the match is (exact constant match vs. near-match),
and show the physical-quantity name. GNU Units has no equivalent.

**7. Nonlinear unit definitions in user config**
The hardest item in this tier but the one that most expands power-user value.
Allow `tempF(x) = x K + 273.15` syntax in `~/.config/runits/units.conf`. Even
a read-only forward-only subset (no inversion) would cover most real needs and
enables user-defined scales (Beaufort, Saffir-Simpson, Mohs hardness).

### Tier 3 — Interesting, narrower audience

**8. WASM playground**
`runits` compiled to WASM + a minimal web UI. Already deferred. Unique
marketing artifact — nobody else in the CLI unit converter space has this.

**9. Significant-figure-aware arithmetic**
Track sig figs through the conversion, emit a warning when the result has
more precision than the input warrants. Already in Extras Catalog. Niche but
beloved by chemistry/physics students.

**10. Constants with CODATA uncertainty**
Physical constants with official uncertainties (`c = 299792458 m/s (exact)`,
`G = 6.67430(15)×10⁻¹¹ m³/(kg·s²)`). GNU Units stores only the central value.
Niche, but highly differentiating for scientific use.

### Explicitly not recommended

**Integer fraction syntax (`1|2 meter`)**
The user's instinct is right — it's confusing. `|` is not a standard operator,
it collides with shell pipe expectations, and `0.5 meter` serves the same need
clearly. GNU Units inherited this from its 1980s origin. Skip it.
