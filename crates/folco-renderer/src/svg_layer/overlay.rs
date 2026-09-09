//! SVG overlay layer — the vector analogue of the raster image overlay.
//!
//! Realizes the medium-neutral [`ImageOverlayConfig`] intent in vector form.
//! Rather than compositing pixels at computed coordinates, the overlay source
//! is embedded as an SVG `<image>` element positioned with percentage
//! coordinates:
//!
//! - **SVG / emoji sources** are embedded as a `data:image/svg+xml` URI,
//!   preserving vector fidelity.
//! - **Raster (PNG) sources** are wrapped in the same `<image>` element via a
//!   `data:image/png` URI.
//!
//! Percentage placement means the layer never needs the base icon's coordinate
//! system (viewBox); `preserveAspectRatio` keeps the source's aspect ratio.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::SvgLayer;
use crate::error::RenderError;
use crate::layer::{ImageOverlayConfig, ImageSource, OverlayAnchorMode, OverlayPosition};

impl SvgLayer<ImageOverlayConfig> {
    /// Renders the overlay as an SVG `<image>` fragment, or `None` if inactive.
    ///
    /// # Errors
    ///
    /// Returns an error if an emoji/SVG source cannot be resolved.
    pub fn render_fragment(&self) -> Result<Option<String>, RenderError> {
        if !self.is_active() {
            return Ok(None);
        }
        let config = self.config().expect("active layer always has a config");
        Ok(Some(overlay_fragment(config)?))
    }
}

/// Builds the `<image>` fragment for an overlay.
fn overlay_fragment(config: &ImageOverlayConfig) -> Result<String, RenderError> {
    let href = source_data_uri(&config.source)?;
    let (x, y, size) = placement(config.position, config.anchor_mode, config.scale);
    Ok(format!(
        "<image href=\"{href}\" x=\"{x:.4}%\" y=\"{y:.4}%\" \
width=\"{size:.4}%\" height=\"{size:.4}%\" preserveAspectRatio=\"xMidYMid meet\"/>"
    ))
}

/// Encodes an image source as a `data:` URI suitable for an `<image href>`.
fn source_data_uri(source: &ImageSource) -> Result<String, RenderError> {
    match source {
        ImageSource::Svg(svg) => {
            let markup = svg.resolve()?;
            Ok(format!(
                "data:image/svg+xml;base64,{}",
                STANDARD.encode(markup.as_bytes())
            ))
        }
        ImageSource::Raster(png) => Ok(format!("data:image/png;base64,{}", STANDARD.encode(png))),
    }
}

/// Computes `(x%, y%, size%)` placement for an overlay of the given scale.
///
/// `scale` is the overlay's edge length as a fraction of the icon. `Inset`
/// keeps the overlay fully within bounds; `Centered` centers it on the chosen
/// corner so it can hang off the edge (via negative offsets).
fn placement(
    position: OverlayPosition,
    anchor_mode: OverlayAnchorMode,
    scale: f32,
) -> (f32, f32, f32) {
    let size = scale.clamp(0.0, 1.0) * 100.0;
    let half = size / 2.0;

    let (x, y) = match (position, anchor_mode) {
        (OverlayPosition::Center, _) => ((100.0 - size) / 2.0, (100.0 - size) / 2.0),

        (OverlayPosition::TopLeft, OverlayAnchorMode::Inset) => (0.0, 0.0),
        (OverlayPosition::TopRight, OverlayAnchorMode::Inset) => (100.0 - size, 0.0),
        (OverlayPosition::BottomLeft, OverlayAnchorMode::Inset) => (0.0, 100.0 - size),
        (OverlayPosition::BottomRight, OverlayAnchorMode::Inset) => (100.0 - size, 100.0 - size),

        (OverlayPosition::TopLeft, OverlayAnchorMode::Centered) => (-half, -half),
        (OverlayPosition::TopRight, OverlayAnchorMode::Centered) => (100.0 - half, -half),
        (OverlayPosition::BottomLeft, OverlayAnchorMode::Centered) => (-half, 100.0 - half),
        (OverlayPosition::BottomRight, OverlayAnchorMode::Centered) => (100.0 - half, 100.0 - half),
    };

    (x, y, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn red_png_source() -> ImageSource {
        let img = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
        ImageSource::from_rgba_image(&img).unwrap()
    }

    #[test]
    fn inactive_layer_renders_nothing() {
        let layer: SvgLayer<ImageOverlayConfig> = SvgLayer::default();
        assert_eq!(layer.render_fragment().unwrap(), None);
    }

    #[test]
    fn svg_source_embeds_as_svg_data_uri() {
        let mut layer: SvgLayer<ImageOverlayConfig> = SvgLayer::default();
        layer.set_config(Some(ImageOverlayConfig::from_svg(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
            OverlayPosition::Center,
            OverlayAnchorMode::Inset,
            0.5,
        )));
        let fragment = layer.render_fragment().unwrap().unwrap();

        assert!(fragment.starts_with("<image"));
        assert!(fragment.contains("data:image/svg+xml;base64,"));
        assert!(fragment.contains("preserveAspectRatio=\"xMidYMid meet\""));
    }

    #[test]
    fn raster_source_wraps_in_image_png_uri() {
        let mut layer: SvgLayer<ImageOverlayConfig> = SvgLayer::default();
        layer.set_config(Some(ImageOverlayConfig::new(
            red_png_source(),
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Inset,
            0.25,
        )));
        let fragment = layer.render_fragment().unwrap().unwrap();

        assert!(fragment.contains("data:image/png;base64,"));
    }

    #[test]
    fn inset_bottom_right_stays_in_bounds() {
        let (x, y, size) = placement(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25);
        assert_eq!(size, 25.0);
        assert_eq!((x, y), (75.0, 75.0));
    }

    #[test]
    fn centered_bottom_right_can_hang_off() {
        let (x, y, size) = placement(
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Centered,
            0.5,
        );
        assert_eq!(size, 50.0);
        // Centered on the corner: half hangs past the 100% edge.
        assert_eq!((x, y), (75.0, 75.0));
    }

    #[test]
    fn centered_top_left_uses_negative_offsets() {
        let (x, y, _) = placement(OverlayPosition::TopLeft, OverlayAnchorMode::Centered, 0.4);
        assert_eq!((x, y), (-20.0, -20.0));
    }
}
