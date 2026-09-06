//! Color-dot layer — the SVG analogue of raster color targeting.
//!
//! Raster folder icons are recolored by HSL-shifting pixels; scalable SVG icons
//! can't be recolored that way, so the folder-color intent is expressed instead
//! as a small colored dot overlaid on the icon.

use super::SvgLayer;
use crate::layer::LayerConfig;

/// Configuration for the SVG color-dot overlay — pure data.
///
/// Holds the RGB folder color. The SVG fragment is produced by
/// [`SvgLayer<ColorDotConfig>::render_fragment`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorDotConfig {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,
}

impl ColorDotConfig {
    /// Creates a new color-dot config from RGB values.
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

impl LayerConfig for ColorDotConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self != other
    }
}

impl SvgLayer<ColorDotConfig> {
    /// Renders the color dot as an SVG fragment, or `None` if inactive.
    ///
    /// The dot is emitted as a nested `<svg>` positioned with percentage
    /// coordinates, so it does not need to know the base icon's coordinate
    /// system (viewBox).
    pub fn render_fragment(&self) -> Option<String> {
        if !self.is_active() {
            return None;
        }
        Some(color_dot_fragment(self.config()?))
    }
}

/// Builds the nested-`<svg>` fragment for a color dot in the bottom-right.
fn color_dot_fragment(config: &ColorDotConfig) -> String {
    let fill = format!("#{:02x}{:02x}{:02x}", config.r, config.g, config.b);
    // A nested viewport in the bottom-right quadrant (55%–95%), sized ~40% of
    // the icon. Percentage placement means we never need the base viewBox.
    // The light ring keeps the dot legible over dark folder art.
    format!(
        "<svg x=\"55%\" y=\"55%\" width=\"40%\" height=\"40%\" \
viewBox=\"0 0 100 100\" preserveAspectRatio=\"xMidYMid meet\">\
<circle cx=\"50\" cy=\"50\" r=\"42\" fill=\"{fill}\" \
stroke=\"#ffffff\" stroke-width=\"8\"/></svg>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_layer_renders_nothing() {
        let layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        assert!(layer.render_fragment().is_none());
    }

    #[test]
    fn disabled_layer_renders_nothing() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(33, 150, 243)));
        layer.set_enabled(false);
        assert!(layer.render_fragment().is_none());
    }

    #[test]
    fn active_layer_renders_circle_with_color() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(33, 150, 243)));
        let fragment = layer.render_fragment().unwrap();

        assert!(fragment.contains("<circle"));
        assert!(fragment.contains("#2196f3"));
        assert!(fragment.starts_with("<svg"));
        assert!(fragment.ends_with("</svg>"));
    }

    #[test]
    fn differs_from_detects_color_change() {
        let a = ColorDotConfig::new(0, 0, 0);
        let b = ColorDotConfig::new(0, 0, 1);
        assert!(a.differs_from(&b));
        assert!(!a.differs_from(&a.clone()));
    }
}
