//! SVG color-dot layer — the vector analogue of the raster color dot.
//!
//! Realizes the medium-neutral [`ColorDotConfig`] intent in vector form: the
//! dot is nested as its own `<svg>` viewport placed with percentage
//! coordinates, so the layer never needs the base icon's coordinate system
//! (viewBox).

use super::SvgPaint;
use super::overlay::percent_rect;
use crate::error::RenderError;
use crate::layer::ColorDotConfig;
use crate::layer::color_dot::{DOT_VIEWBOX, color_dot_circle};

impl SvgPaint for ColorDotConfig {
    /// The dot as a nested `<svg>` positioned with percentage coordinates, so
    /// it does not need to know the base icon's coordinate system (viewBox).
    fn fragment(&self) -> Result<String, RenderError> {
        let (x, y, size) = percent_rect(&self.placement);
        Ok(format!(
            "<svg x=\"{x:.4}%\" y=\"{y:.4}%\" width=\"{size:.4}%\" height=\"{size:.4}%\" \
viewBox=\"{DOT_VIEWBOX}\" preserveAspectRatio=\"xMidYMid meet\">{}</svg>",
            color_dot_circle(self)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{OverlayAnchorMode, OverlayPosition};
    use crate::placement::Placement;
    use crate::svg_layer::{SvgLayer, SvgStage};

    #[test]
    fn inactive_layer_renders_nothing() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(None);
        assert!(layer.fragment().unwrap().is_none());
    }

    #[test]
    fn active_layer_renders_circle_with_color() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(33, 150, 243)));
        let fragment = layer.fragment().unwrap().unwrap();

        assert!(fragment.contains("<circle"));
        assert!(fragment.contains("#2196f3"));
        assert!(fragment.starts_with("<svg"));
        assert!(fragment.ends_with("</svg>"));
    }

    #[test]
    fn default_fragment_sits_in_the_bottom_right_corner() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(0, 0, 0)));
        let fragment = layer.fragment().unwrap().unwrap();

        assert!(fragment.contains("x=\"60.0000%\""));
        assert!(fragment.contains("width=\"40.0000%\""));
    }

    #[test]
    fn fragment_follows_configured_placement() {
        let mut layer: SvgLayer<ColorDotConfig> = SvgLayer::default();
        layer.set_config(Some(ColorDotConfig::new(0, 0, 0).with_placement(
            Placement::new(OverlayPosition::TopLeft, OverlayAnchorMode::Centered, 0.2),
        )));
        let fragment = layer.fragment().unwrap().unwrap();

        assert!(fragment.contains("x=\"-10.0000%\""));
        assert!(fragment.contains("y=\"-10.0000%\""));
        assert!(fragment.contains("width=\"20.0000%\""));
    }
}
