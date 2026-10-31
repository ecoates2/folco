//! SVG color-dot layer — the vector analogue of the raster color dot.
//!
//! Realizes the medium-neutral [`ColorDotConfig`] intent in vector form: the
//! dot is nested as its own `<svg>` viewport placed with percentage
//! coordinates, so the layer never needs the base icon's coordinate system
//! (viewBox).

use super::SvgLayer;
use super::overlay::percent_rect;
use crate::layer::ColorDotConfig;
use crate::layer::color_dot::{DOT_VIEWBOX, color_dot_circle};

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

/// Builds the nested-`<svg>` fragment for a color dot at its configured placement.
fn color_dot_fragment(config: &ColorDotConfig) -> String {
    let (x, y, size) = percent_rect(&config.placement);
    format!(
        "<svg x=\"{x:.4}%\" y=\"{y:.4}%\" width=\"{size:.4}%\" height=\"{size:.4}%\" \
viewBox=\"{DOT_VIEWBOX}\" preserveAspectRatio=\"xMidYMid meet\">{}</svg>",
        color_dot_circle(config)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{OverlayAnchorMode, OverlayPosition};
    use crate::placement::Placement;

    #[test]
    fn inactive_layer_renders_nothing() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(None);
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
    fn default_fragment_sits_in_the_bottom_right_corner() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(0, 0, 0)));
        let fragment = layer.render_fragment().unwrap();

        assert!(fragment.contains("x=\"60.0000%\""));
        assert!(fragment.contains("width=\"40.0000%\""));
    }

    #[test]
    fn fragment_follows_configured_placement() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(0, 0, 0).with_placement(
            Placement::new(OverlayPosition::TopLeft, OverlayAnchorMode::Centered, 0.2),
        )));
        let fragment = layer.render_fragment().unwrap();

        assert!(fragment.contains("x=\"-10.0000%\""));
        assert!(fragment.contains("y=\"-10.0000%\""));
        assert!(fragment.contains("width=\"20.0000%\""));
    }
}
