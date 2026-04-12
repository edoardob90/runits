//! End-to-end CLI tests via `assert_cmd`.
//!
//! Each test spawns the compiled `runits` binary as a subprocess, feeds it
//! args, and asserts on stdout / stderr / exit status. These are the real
//! "did I break the user's experience?" tests — unit tests prove logic;
//! these prove the binary.

use assert_cmd::Command;
use predicates::prelude::*;

fn runits() -> Command {
    Command::cargo_bin("runits").expect("binary `runits` was not built")
}

// ---- Happy path ----------------------------------------------------------

#[test]
fn basic_conversion_feet_to_meters() {
    runits()
        .arg("10 ft")
        .arg("m")
        .assert()
        .success()
        .stdout(predicate::str::contains("3.048"));
}

#[test]
fn alias_miles_to_kilometers() {
    runits()
        .arg("5 miles")
        .arg("km")
        .assert()
        .success()
        .stdout(predicate::str::contains("8.04"));
}

#[test]
fn short_names_work_both_directions() {
    runits()
        .arg("10 m")
        .arg("ft")
        .assert()
        .success()
        .stdout(predicate::str::contains("32.8"));
}

#[test]
fn scientific_notation_round_trip() {
    runits()
        .arg("6.022e23 mole")
        .arg("mole")
        .assert()
        .success()
        // 6.022e23 formatted via f64 Display becomes the expanded form;
        // we just check the significant digits land in stdout.
        .stdout(predicate::str::contains("6022").or(predicate::str::contains("6.022e23")));
}

#[test]
fn compound_alias_km_h_to_mph() {
    runits()
        .arg("100 km/h")
        .arg("mph")
        .assert()
        .success()
        .stdout(predicate::str::contains("62.1"));
}

// ---- Bare unit names (implicit value = 1) --------------------------------

#[test]
fn bare_unit_name_mile_to_km() {
    // "mile" alone = 1 mile, the canonical "how many km in a mile?" query.
    runits()
        .arg("mile")
        .arg("km")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.60934"));
}

#[test]
fn bare_unit_name_byte_to_bit() {
    runits()
        .arg("byte")
        .arg("bit")
        .assert()
        .success()
        .stdout(predicate::str::contains("8"));
}

#[test]
fn bare_compound_alias_works() {
    runits()
        .arg("km/h")
        .arg("mph")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.6213"));
}

// ---- Output formatting sanity --------------------------------------------

#[test]
fn tiny_values_render_in_scientific_notation() {
    // 50 µs → s gives 5e-5, which should render as "5e-5", not the
    // floating-point-noisy "0.000049999999999999996 second".
    runits()
        .arg("50 µs")
        .arg("s")
        .assert()
        .success()
        .stdout(predicate::str::contains("5e-5"))
        .stdout(predicate::str::contains("0.00004999").not());
}

// ---- Failure paths -------------------------------------------------------

#[test]
fn incompatible_dimensions_fails_with_helpful_message() {
    runits()
        .arg("10 m")
        .arg("kg")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("incompatible dimensions"))
        .stderr(predicate::str::contains("Length"))
        .stderr(predicate::str::contains("Mass"));
}

#[test]
fn unknown_source_unit_fails() {
    // Source-side unknown names now route through the expression evaluator,
    // which tries both units AND constants — so the error is
    // `unknown identifier`, not `unknown unit`. The suggestion fallback
    // (Phase 5a) merges suggestions from both databases.
    runits()
        .arg("10 foozle")
        .arg("m")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown identifier"))
        .stderr(predicate::str::contains("foozle"));
}

#[test]
fn unknown_target_unit_fails() {
    runits()
        .arg("10 m")
        .arg("foozle")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown unit"))
        .stderr(predicate::str::contains("foozle"));
}

#[test]
fn typo_suggests_correction() {
    runits()
        .arg("10 meterr")
        .arg("m")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Did you mean"));
}

#[test]
fn bare_number_is_dimensionless_not_a_length() {
    // Phase 5a: `10` alone is now a valid dimensionless expression. The
    // failure mode shifted from "parse error" to "incompatible dimensions":
    // you can't convert a pure number to meters.
    runits()
        .arg("10")
        .arg("m")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("incompatible dimensions"));
}

#[test]
fn missing_target_arg_is_usage_error() {
    // With optional positionals, giving only one arg hits our usage message.
    runits()
        .arg("10 m")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Usage"));
}

// ---- Temperature conversions (Phase 3) ----

#[test]
fn celsius_to_fahrenheit() {
    runits()
        .arg("100 degC")
        .arg("degF")
        .assert()
        .success()
        .stdout(predicate::str::contains("212"));
}

#[test]
fn fahrenheit_to_celsius() {
    runits()
        .arg("98.6 degF")
        .arg("degC")
        .assert()
        .success()
        .stdout(predicate::str::contains("37"));
}

#[test]
fn kelvin_to_celsius() {
    runits()
        .arg("0 kelvin")
        .arg("degC")
        .assert()
        .success()
        .stdout(predicate::str::contains("-273.15"));
}

#[test]
fn degree_sign_alias_works() {
    runits()
        .arg("100 °C")
        .arg("°F")
        .assert()
        .success()
        .stdout(predicate::str::contains("212"));
}

// ---- --explain flag (Phase 5a) ----

#[test]
fn explain_simple_linear() {
    // 10 ft → m: target is base, so only `source:` appears.
    runits()
        .arg("--explain")
        .arg("10 ft")
        .arg("m")
        .assert()
        .success()
        .stdout(predicate::str::contains("source:"))
        .stdout(predicate::str::contains("target:").not())
        .stdout(predicate::str::contains("0.3048"))
        .stdout(predicate::str::contains("3.048"));
}

#[test]
fn explain_compound_linear() {
    runits()
        .arg("--explain")
        .arg("100 km/h")
        .arg("mph")
        .assert()
        .success()
        .stdout(predicate::str::contains("source:"))
        .stdout(predicate::str::contains("target:"))
        .stdout(predicate::str::contains("62.1371"));
}

#[test]
fn explain_affine_temperature_harmonized_labels() {
    // Affine conversions use the same `source:` / `target:` labels as linear,
    // not the old `to base:` / `from base:` style.
    runits()
        .arg("--explain")
        .arg("98.6 degF")
        .arg("degC")
        .assert()
        .success()
        .stdout(predicate::str::contains("source:"))
        .stdout(predicate::str::contains("target:"))
        .stdout(predicate::str::contains("to base:").not())
        .stdout(predicate::str::contains("from base:").not())
        .stdout(predicate::str::contains("37"));
}

// ---- Expression foundation (Phase 5a) ----

#[test]
fn expression_implicit_multiplication() {
    // `3*4 m` → 12 m → ~39.37 ft
    runits()
        .arg("3*4 meter")
        .arg("foot")
        .assert()
        .success()
        .stdout(predicate::str::contains("39.3"));
}

#[test]
fn expression_pow_then_juxtapose() {
    // `2^10 byte` → 1024 byte → 1.024 kB
    runits()
        .arg("2^10 byte")
        .arg("kB")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.024"));
}

#[test]
fn expression_addition_same_dim() {
    // `5 m + 3 ft` → 5.9144 m → 591.44 cm
    runits()
        .arg("5 m + 3 ft")
        .arg("cm")
        .assert()
        .success()
        .stdout(predicate::str::contains("591.4"));
}

#[test]
fn expression_addition_incompatible() {
    runits()
        .arg("5 m + 3 s")
        .arg("cm")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Length"))
        .stderr(predicate::str::contains("Time"));
}

#[test]
fn expression_constant_arithmetic() {
    // `2 * c_0 * 1 s` ≈ 599584.916 km
    runits()
        .arg("2 * c_0 * 1 s")
        .arg("km")
        .assert()
        .success()
        .stdout(predicate::str::contains("599584").or(predicate::str::contains("599585")));
}

#[test]
fn expression_sqrt_dim_transform() {
    // `sqrt(9 m^2)` → 3 m → 300 cm
    runits()
        .arg("sqrt(9 m^2)")
        .arg("cm")
        .assert()
        .success()
        .stdout(predicate::str::contains("300"));
}

#[test]
fn expression_sqrt_odd_exponent_fails() {
    runits()
        .arg("sqrt(9 m)")
        .arg("cm")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("sqrt"));
}

#[test]
fn expression_sin_scalar_ok() {
    // `sin(0) m` → 0 m → 0 cm
    runits()
        .arg("sin(0) m")
        .arg("cm")
        .assert()
        .success()
        .stdout(predicate::str::contains("0"));
}

#[test]
fn expression_sin_dimensioned_fails() {
    runits()
        .arg("sin(5 m)")
        .arg("cm")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("sin"));
}

#[test]
fn expression_unknown_function_suggests() {
    runits()
        .arg("sxrt(9 m^2)")
        .arg("cm")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("sqrt"));
}

#[test]
fn expression_negation() {
    // `--` separates flags from positionals so clap doesn't interpret `-5`
    // as a short flag.
    runits()
        .arg("--")
        .arg("-5 m")
        .arg("cm")
        .assert()
        .success()
        .stdout(predicate::str::contains("-500"));
}

#[test]
fn expression_affine_addition_fails() {
    runits()
        .arg("20 celsius + 5 celsius")
        .arg("K")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("celsius"));
}

#[test]
fn expression_underscore_prefix_fails_cleanly() {
    // `_foo` must not be a partial `_` + dangling `foo` — it should fail
    // the grammar at the parser level.
    runits().arg("_foo m").arg("cm").assert().failure().code(1);
}

#[test]
fn explain_preserves_expression_source() {
    // --explain on a non-trivial expression should echo the original input
    // on an `expression:` line.
    runits()
        .arg("--explain")
        .arg("5 m + 3 ft")
        .arg("cm")
        .assert()
        .success()
        .stdout(predicate::str::contains("expression:"))
        .stdout(predicate::str::contains("5 m + 3 ft"));
}

#[test]
fn explain_trivial_conversion_skips_expression_row() {
    // A simple `10 ft` should NOT get an "expression:" row — the source
    // row already shows "10 foot" which is the same information.
    runits()
        .arg("--explain")
        .arg("10 ft")
        .arg("m")
        .assert()
        .success()
        .stdout(predicate::str::contains("expression:").not());
}

// ---- REPL previous-result chain ----

#[test]
fn repl_previous_result_chain() {
    // Feed a three-line REPL session: establish a quantity, use `_` to
    // double it, then convert via `_ to km`. Assert that the printed
    // values form the expected sequence.
    let mut cmd = runits();
    cmd.env("NO_COLOR", "1")
        .env_remove("CLICOLOR")
        .arg("--intro-banner")
        .arg("off")
        .write_stdin("5 m + 3 ft\n_ * 2\n_ to km\nquit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("5.9144"))
        .stdout(predicate::str::contains("11.8288"))
        .stdout(predicate::str::contains("0.0118288"));
}

#[test]
fn repl_previous_unavailable_errors_cleanly() {
    let mut cmd = runits();
    cmd.env("NO_COLOR", "1")
        .arg("--intro-banner")
        .arg("off")
        .write_stdin("_ + 5 m\nquit\n")
        .assert()
        .success() // REPL doesn't exit on error, it prints and continues
        .stderr(predicate::str::contains("no previous result"));
}
