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
    runits()
        .arg("10 foozle")
        .arg("m")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("unknown unit"))
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
fn bare_number_without_unit_fails_with_parse_error() {
    // "10" alone fails the grammar: either it's a number that needs a
    // following unit_name (missing whitespace+unit), or it's a unit_name
    // that must start with a letter. Neither branch matches → parse error.
    runits()
        .arg("10")
        .arg("m")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("parse error"));
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
    runits()
        .arg("--explain")
        .arg("10 ft")
        .arg("m")
        .assert()
        .success()
        .stdout(predicate::str::contains("factor:"))
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
fn explain_affine_temperature() {
    runits()
        .arg("--explain")
        .arg("98.6 degF")
        .arg("degC")
        .assert()
        .success()
        .stdout(predicate::str::contains("to base:"))
        .stdout(predicate::str::contains("from base:"))
        .stdout(predicate::str::contains("37"));
}
