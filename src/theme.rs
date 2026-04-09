//! Dimension-based color theme for consistent styling.
//!
//! Each base dimension has its own color. Units inherit color from their
//! dimension (single-dimension) or use the compound style (multi-dimension).
//! Flexoki-inspired (https://github.com/kepano/flexoki) ANSI defaults.
//! FUTURE(theme-config): loadable from config.toml

use crate::units::dimension::Dimension;
use owo_colors::Style;

/// Semantic color roles for the entire output pipeline.
///
/// A unit name is always styled by its dimension — whether it appears in a
/// conversion result, `?` help, or REPL highlighting. The `color` flag
/// controls whether styles are actually applied (respects `NO_COLOR`, piping).
#[derive(Debug, Clone)]
pub struct Theme {
    // Color mode toggle
    pub color: bool,
    // Per-dimension colors
    pub length: Style,
    pub mass: Style,
    pub time: Style,
    pub temperature: Style,
    pub current: Style,
    pub amount: Style,
    pub intensity: Style,
    pub angle: Style,
    pub information: Style,
    pub currency: Style,
    // Compound/derived quantity color (Force, Velocity, etc.)
    pub compound: Style,
    // Utility styles
    pub number: Style,
    pub keyword: Style,
    pub constant: Style,
    pub dimmed: Style,
    pub error: Style,
}

impl Theme {
    /// Default constructor for a Theme.
    pub fn new(color: bool) -> Self {
        Theme {
            color,
            length: Style::new().blue(),
            mass: Style::new().red(),
            time: Style::new().green(),
            temperature: Style::new().truecolor(218, 112, 44), // Flexoki orange
            current: Style::new().yellow(),
            amount: Style::new().magenta(),
            intensity: Style::new().bright_magenta(),
            angle: Style::new().cyan(),
            information: Style::new().bright_blue(),
            currency: Style::new().bright_yellow(),
            compound: Style::new().bright_white().bold(),
            number: Style::new().yellow(),
            keyword: Style::new().dimmed().bold(),
            constant: Style::new().truecolor(175, 135, 255).italic(), // Flexoki purple, italic
            dimmed: Style::new().dimmed(),
            error: Style::new().red(),
        }
    }

    /// Apply a style to text, respecting color enable flag.
    pub fn paint(&self, text: &str, style: &Style) -> String {
        if self.color {
            format!("{}", style.style(text))
        } else {
            text.to_string()
        }
    }

    /// Style for a specific dimension.
    pub fn dimension_style(&self, dim: &Dimension) -> &Style {
        match dim {
            Dimension::Length => &self.length,
            Dimension::Mass => &self.mass,
            Dimension::Time => &self.time,
            Dimension::Temperature => &self.temperature,
            Dimension::Current => &self.current,
            Dimension::AmountOfSubstance => &self.amount,
            Dimension::LuminousIntensity => &self.intensity,
            Dimension::Angle => &self.angle,
            Dimension::Information => &self.information,
            Dimension::Currency => &self.currency,
        }
    }

    /// Style for a dimension signature.
    /// Single-dimension → that dimension's color.
    /// Multi-dimension (compound) or dimensionless → compound style.
    pub fn dims_style(&self, dims: &crate::units::dimension::DimensionMap) -> &Style {
        if dims.len() == 1 {
            let (dim, _) = dims.iter().next().unwrap();
            self.dimension_style(dim)
        } else {
            &self.compound
        }
    }

    /// Style for a unit based on its dimensions. Delegates to [`dims_style`].
    pub fn unit_style(&self, unit: &crate::units::Unit) -> &Style {
        self.dims_style(&unit.dimensions)
    }

    // Convenience methods.
    pub fn unit_text(&self, text: &str, unit: &crate::units::Unit) -> String {
        self.paint(text, self.unit_style(unit))
    }
    pub fn num(&self, text: &str) -> String {
        self.paint(text, &self.number)
    }
    pub fn kw(&self, text: &str) -> String {
        self.paint(text, &self.keyword)
    }
    pub fn lbl(&self, text: &str) -> String {
        self.paint(text, &self.compound) // labels use compound/bold style
    }
    pub fn cst(&self, text: &str) -> String {
        self.paint(text, &self.constant)
    }
    pub fn dim(&self, text: &str) -> String {
        self.paint(text, &self.dimmed)
    }
    pub fn err(&self, text: &str) -> String {
        self.paint(text, &self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_paint_no_color() {
        let t = Theme::new(false);
        let meter = crate::units::Unit::meter();
        assert_eq!(t.unit_text("meter", &meter), "meter");
        assert_eq!(t.num("3.14"), "3.14");
    }
}
