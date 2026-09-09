//! SVG color-dot layer — the vector analogue of the raster color dot.
//!
//! Realizes the medium-neutral [`ColorDotConfig`] intent in vector form: the
//! dot is nested as its own `<svg>` viewport placed with percentage
//! coordinates, so the layer never needs the base icon's coordinate system
//! (viewBox).

use super::SvgLayer;
use crate::layer::ColorDotConfig;
use crate::layer::color_dot::{DOT_MARGIN, DOT_SCALE, DOT_VIEWBOX, color_dot_circle};

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
    let size = DOT_SCALE * 100.0;
    let offset = (1.0 - DOT_SCALE - DOT_MARGIN) * 100.0;
    format!(
        "<svg x=\"{offset}%\" y=\"{offset}%\" width=\"{size}%\" height=\"{size}%\" \
viewBox=\"{DOT_VIEWBOX}\" preserveAspectRatio=\"xMidYMid meet\">{}</svg>",
        color_dot_circle(config)
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
    fn fragment_is_inset_from_the_bottom_right_corner() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(0, 0, 0)));
        let fragment = layer.render_fragment().unwrap();

        assert!(fragment.contains("x=\"55%\""));
        assert!(fragment.contains("width=\"40%\""));
    }
}
