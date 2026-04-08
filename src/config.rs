//! Optional TOML configuration file at `~/.config/runits/config.toml`.
//!
//! All fields are optional — missing file or missing keys use sensible defaults.
//! CLI flags always override config values.

use serde::Deserialize;
use std::path::PathBuf;

/// User configuration deserialized from TOML.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Default significant figures (overridden by `--precision`).
    pub precision: Option<usize>,
    /// Enable/disable ANSI colors (overridden by `NO_COLOR` env).
    pub color: Option<bool>,
    /// Enable/disable Unicode unit rendering (overridden by `--pretty`).
    pub unicode: Option<bool>,
    /// Intro banner mode: "long", "short", or "off".
    pub intro_banner: Option<String>,
}

impl Config {
    /// Load config from `~/.config/runits/config.toml`.
    ///
    /// Returns `Config::default()` if the file doesn't exist or can't be parsed.
    /// Prints a warning to stderr on parse errors.
    pub fn load() -> Self {
        let Some(path) = config_path() else {
            return Config::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str(&contents) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("Warning: malformed config at {}: {e}", path.display());
                    Config::default()
                }
            },
            Err(_) => Config::default(), // File doesn't exist — not an error.
        }
    }
}

/// Returns `~/.config/runits/config.toml`.
fn config_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        Some(PathBuf::from(xdg).join("runits").join("config.toml"))
    } else {
        std::env::var("HOME").ok().map(|h| {
            PathBuf::from(h)
                .join(".config")
                .join("runits")
                .join("config.toml")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_all_none() {
        let c = Config::default();
        assert!(c.precision.is_none());
        assert!(c.color.is_none());
        assert!(c.unicode.is_none());
    }

    #[test]
    fn parse_valid_toml() {
        let toml_str = r#"
            precision = 8
            color = true
            unicode = false
        "#;
        let c: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(c.precision, Some(8));
        assert_eq!(c.color, Some(true));
        assert_eq!(c.unicode, Some(false));
    }

    #[test]
    fn parse_partial_toml() {
        let toml_str = "precision = 3\n";
        let c: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(c.precision, Some(3));
        assert!(c.color.is_none());
    }
}
