//! Builtin unit definitions: ~63 units with aliases, grouped by dimension.

use crate::units::Unit;
use crate::units::dimension::Dimension;
use std::collections::HashMap;

/// Helper: register every `alias` as a lookup key for `unit`.
fn add(map: &mut HashMap<String, Unit>, aliases: &[&str], unit: Unit) {
    for alias in aliases {
        map.insert((*alias).to_string(), unit.clone());
    }
}

/// Rename a unit in-place (the `Mul`/`Div` impls auto-generate names like
/// `"meter/second"` — which is already what we want, but the helper makes
/// the intent explicit at the call site).
fn rename(mut unit: Unit, new_name: &str) -> Unit {
    unit.name = new_name.to_string();
    unit
}

/// Seeds the ~80 Phase 2 entries. Grouped by dimension for readability.
pub(super) fn seed_all(map: &mut HashMap<String, Unit>) {
    // ---- SI base units + base extensions ----
    add(map, &["meter", "m", "meters", "metres"], Unit::meter());
    add(map, &["kilogram", "kg", "kilograms"], Unit::kilogram());
    add(map, &["second", "s", "sec", "seconds"], Unit::second());
    add(map, &["ampere", "A", "amps", "amperes"], Unit::ampere());
    add(map, &["kelvin", "K"], Unit::kelvin());
    // Absolute temperature scales (affine conversions)
    add(map, &["celsius", "degC", "°C"], Unit::celsius());
    add(map, &["fahrenheit", "degF", "°F"], Unit::fahrenheit());
    add(map, &["rankine", "Ra"], Unit::rankine());
    add(map, &["reaumur", "Re", "°Re"], Unit::reaumur());
    add(map, &["mole", "mol", "moles"], Unit::mole());
    add(map, &["candela", "cd"], Unit::candela());
    add(map, &["radian", "rad", "radians"], Unit::radian());
    add(map, &["bit", "b", "bits"], Unit::bit());

    // ---- Length ----
    add(
        map,
        &["kilometer", "km", "kilometers", "kilometres"],
        Unit::kilometer(),
    );
    add(
        map,
        &["centimeter", "cm", "centimeters", "centimetres"],
        Unit::new("centimeter", 0.01, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["millimeter", "mm", "millimeters", "millimetres"],
        Unit::new("millimeter", 0.001, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["micrometer", "µm", "um", "micrometers", "micrometres"],
        Unit::new("micrometer", 1e-6, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["nanometer", "nm", "nanometers", "nanometres"],
        Unit::new("nanometer", 1e-9, &[(Dimension::Length, 1)]),
    );
    add(map, &["mile", "mi", "miles"], Unit::mile());
    add(map, &["foot", "ft", "feet"], Unit::foot());
    add(map, &["inch", "in", "inches"], Unit::inch());
    add(
        map,
        &["yard", "yd", "yards"],
        Unit::new("yard", 0.9144, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["nautical_mile", "nmi"],
        Unit::new("nautical_mile", 1852.0, &[(Dimension::Length, 1)]),
    );

    // ---- Time ----
    add(map, &["minute", "min", "minutes"], Unit::minute());
    add(map, &["hour", "hr", "h", "hours"], Unit::hour());
    add(
        map,
        &["day", "d", "days"],
        Unit::new("day", 86_400.0, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["week", "wk", "weeks"],
        Unit::new("week", 604_800.0, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["year", "yr", "years"],
        // Julian year (365.25 days); good enough for Phase 2 scope.
        Unit::new("year", 31_557_600.0, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["millisecond", "ms", "milliseconds"],
        Unit::new("millisecond", 1e-3, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["microsecond", "µs", "us", "microseconds"],
        Unit::new("microsecond", 1e-6, &[(Dimension::Time, 1)]),
    );
    add(
        map,
        &["nanosecond", "ns", "nanoseconds"],
        Unit::new("nanosecond", 1e-9, &[(Dimension::Time, 1)]),
    );

    // ---- Mass ----
    add(
        map,
        &["gram", "g", "grams"],
        Unit::new_si("gram", 1e-3, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["milligram", "mg", "milligrams"],
        Unit::new("milligram", 1e-6, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["tonne", "t", "tonnes", "metric_ton"],
        Unit::new("tonne", 1000.0, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["pound", "lb", "lbs", "pounds"],
        Unit::new("pound", 0.45359237, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["ounce", "oz", "ounces"],
        Unit::new("ounce", 0.028349523125, &[(Dimension::Mass, 1)]),
    );
    add(
        map,
        &["stone"],
        Unit::new("stone", 6.35029318, &[(Dimension::Mass, 1)]),
    );

    // ---- Volume (Length^3) ----
    // 1 liter = 1e-3 m^3
    add(
        map,
        &["liter", "l", "L", "litre", "liters", "litres"],
        Unit::new_si("liter", 1e-3, &[(Dimension::Length, 3)]),
    );
    add(
        map,
        &["milliliter", "ml", "mL", "millilitre"],
        Unit::new("milliliter", 1e-6, &[(Dimension::Length, 3)]),
    );
    // US gallon = 3.785411784 liters = 3.785411784e-3 m^3
    add(
        map,
        &["gallon", "gal", "gallons"],
        Unit::new("gallon", 3.785411784e-3, &[(Dimension::Length, 3)]),
    );
    // US cup = 236.5882365 mL
    add(
        map,
        &["cup", "cups"],
        Unit::new("cup", 2.365882365e-4, &[(Dimension::Length, 3)]),
    );
    // US fluid ounce
    add(
        map,
        &["fluid_ounce", "fl_oz"],
        Unit::new("fluid_ounce", 2.95735295625e-5, &[(Dimension::Length, 3)]),
    );

    // ---- Angles ----
    add(map, &["degree", "deg", "degrees"], Unit::degree());
    add(
        map,
        &["arcminute", "arcmin"],
        // 1 arcmin = (1/60) degree = (π / 10800) rad
        Unit::new(
            "arcminute",
            std::f64::consts::PI / 10_800.0,
            &[(Dimension::Angle, 1)],
        ),
    );
    add(
        map,
        &["arcsecond", "arcsec"],
        Unit::new(
            "arcsecond",
            std::f64::consts::PI / 648_000.0,
            &[(Dimension::Angle, 1)],
        ),
    );

    // ---- Information ----
    add(map, &["byte", "B", "bytes"], Unit::byte());
    add(
        map,
        &["kilobyte", "KB", "kB"],
        // Decimal kilobyte: 1000 bytes = 8000 bits
        Unit::new("kilobyte", 8_000.0, &[(Dimension::Information, 1)]),
    );
    add(
        map,
        &["megabyte", "MB"],
        Unit::new("megabyte", 8_000_000.0, &[(Dimension::Information, 1)]),
    );
    add(
        map,
        &["gigabyte", "GB"],
        Unit::new("gigabyte", 8_000_000_000.0, &[(Dimension::Information, 1)]),
    );
    add(
        map,
        &["terabyte", "TB"],
        Unit::new(
            "terabyte",
            8_000_000_000_000.0,
            &[(Dimension::Information, 1)],
        ),
    );

    // ---- Frequency (Time^-1) ----
    // 1 Hz = 1 / second
    add(
        map,
        &["hertz", "Hz"],
        Unit::new_si("hertz", 1.0, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["kilohertz", "kHz"],
        Unit::new("kilohertz", 1e3, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["megahertz", "MHz"],
        Unit::new("megahertz", 1e6, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["gigahertz", "GHz"],
        Unit::new("gigahertz", 1e9, &[(Dimension::Time, -1)]),
    );

    // ---- Compound aliases (pre-built via Unit arithmetic) ----
    // These give users velocity / frequency conversions despite Phase 2's
    // grammar not yet supporting compound parsing. Phase 3 will obsolete the
    // "pre-built" trick in favor of dynamic parsing — but the aliases can
    // stay as handy shortcuts.
    let m_per_s = Unit::meter() / Unit::second();
    add(map, &["m/s", "mps"], rename(m_per_s, "meter/second"));

    let km_per_h = Unit::kilometer() / Unit::hour();
    add(
        map,
        &["km/h", "kph", "kmh"],
        rename(km_per_h, "kilometer/hour"),
    );

    let mph = Unit::mile() / Unit::hour();
    add(map, &["mph", "mi/h"], rename(mph, "mile/hour"));

    // rpm: 1 revolution per minute = (1/60) Hz. A revolution is treated as
    // dimensionless (like radians — technically an angle, but idiomatically
    // folded into frequency here).
    add(
        map,
        &["rpm"],
        Unit::new("rpm", 1.0 / 60.0, &[(Dimension::Time, -1)]),
    );

    // ---- Force (M·L·T⁻²) ----
    let force_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, 1),
        (Dimension::Time, -2),
    ];
    add(
        map,
        &["newton", "N", "newtons"],
        Unit::new_si("newton", 1.0, force_dims),
    );
    add(map, &["dyne", "dyn"], Unit::new("dyne", 1e-5, force_dims));
    add(
        map,
        &["pound_force", "lbf"],
        Unit::new("pound_force", 4.448222, force_dims),
    );
    add(
        map,
        &["kilogram_force", "kgf"],
        Unit::new("kilogram_force", 9.80665, force_dims),
    );

    // ---- Pressure (M·L⁻¹·T⁻²) ----
    let pressure_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, -1),
        (Dimension::Time, -2),
    ];
    add(
        map,
        &["pascal", "Pa", "pascals"],
        Unit::new_si("pascal", 1.0, pressure_dims),
    );
    add(map, &["bar", "bars"], Unit::new("bar", 1e5, pressure_dims));
    add(
        map,
        &["atmosphere", "atm"],
        Unit::new("atmosphere", 101_325.0, pressure_dims),
    );
    add(
        map,
        &["torr", "Torr"],
        Unit::new("torr", 101_325.0 / 760.0, pressure_dims),
    );
    add(
        map,
        &["mmHg"],
        Unit::new("mmHg", 133.322387415, pressure_dims),
    );
    add(
        map,
        &["psi", "PSI"],
        Unit::new("psi", 6894.757, pressure_dims),
    );
    add(map, &["inHg"], Unit::new("inHg", 3386.389, pressure_dims));

    // ---- Energy (M·L²·T⁻²) ----
    let energy_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, 2),
        (Dimension::Time, -2),
    ];
    add(
        map,
        &["joule", "J", "joules"],
        Unit::new_si("joule", 1.0, energy_dims),
    );
    add(
        map,
        &["calorie", "cal", "calories"],
        Unit::new("calorie", 4.184, energy_dims),
    );
    add(
        map,
        &["BTU", "Btu"],
        Unit::new("BTU", 1055.05585262, energy_dims),
    );
    add(
        map,
        &["kilowatt_hour", "kWh"],
        Unit::new("kilowatt_hour", 3.6e6, energy_dims),
    );
    add(
        map,
        &["electronvolt", "eV"],
        Unit::new_si("electronvolt", 1.602_176_634e-19, energy_dims),
    );

    // ---- Power (M·L²·T⁻³) ----
    let power_dims = &[
        (Dimension::Mass, 1),
        (Dimension::Length, 2),
        (Dimension::Time, -3),
    ];
    add(
        map,
        &["watt", "W", "watts"],
        Unit::new_si("watt", 1.0, power_dims),
    );
    add(
        map,
        &["horsepower", "hp"],
        Unit::new("horsepower", 735.49875, power_dims),
    );

    // ---- Historical length ----
    add(
        map,
        &["furlong", "furlongs"],
        Unit::new("furlong", 201.168, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["league", "leagues"],
        Unit::new("league", 4828.032, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["fathom", "fathoms"],
        Unit::new("fathom", 1.8288, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["rod", "rods"],
        Unit::new("rod", 5.0292, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["chain", "chains"],
        Unit::new("chain", 20.1168, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["cubit", "cubits"],
        Unit::new("cubit", 0.4572, &[(Dimension::Length, 1)]),
    );

    // ---- Cooking (volume) ----
    // Volume dims: L³
    let volume_dims = &[(Dimension::Length, 3)];
    add(
        map,
        &["tablespoon", "tbsp"],
        Unit::new("tablespoon", 1.5e-5, volume_dims),
    );
    add(
        map,
        &["teaspoon", "tsp"],
        Unit::new("teaspoon", 5e-6, volume_dims),
    );

    // ---- Astronomical ----
    add(
        map,
        &["astronomical_unit", "AU", "au"],
        Unit::new(
            "astronomical_unit",
            1.495_978_707e11,
            &[(Dimension::Length, 1)],
        ),
    );
    add(
        map,
        &["light_year", "ly"],
        Unit::new("light_year", 9.4607e15, &[(Dimension::Length, 1)]),
    );
    add(
        map,
        &["parsec", "pc"],
        Unit::new("parsec", 3.0857e16, &[(Dimension::Length, 1)]),
    );

    // ---- Radioactivity ----
    add(
        map,
        &["becquerel", "Bq"],
        Unit::new_si("becquerel", 1.0, &[(Dimension::Time, -1)]),
    );
    add(
        map,
        &["curie", "Ci"],
        Unit::new("curie", 3.7e10, &[(Dimension::Time, -1)]),
    );
    // Absorbed dose (L²·T⁻²)
    let dose_dims = &[(Dimension::Length, 2), (Dimension::Time, -2)];
    add(map, &["gray", "Gy"], Unit::new_si("gray", 1.0, dose_dims));
    add(
        map,
        &["sievert", "Sv"],
        Unit::new_si("sievert", 1.0, dose_dims),
    );
}
