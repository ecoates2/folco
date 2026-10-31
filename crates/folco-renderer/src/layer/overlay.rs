//! Image overlay layer — configuration and rendering.
//!
//! Supports both vector (SVG/emoji) and raster (PNG) overlay sources
//! via [`ImageSource`].

use super::image_source::ImageSource;
use super::svg::{SvgSource, composite_over};
use super::{CacheKey, DependencyVersion, Layer, LayerConfig, RenderContext};
use crate::error::RenderError;
use crate::icon::RectPx;
use crate::placement::Placement;
use image::RgbaImage;

// ============================================================================
// OverlayPosition
// ============================================================================

/// Position for image overlay placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "kebab-case")]
pub enum OverlayPosition {
    /// Bottom-left corner of content bounds.
    BottomLeft,
    /// Bottom-right corner of content bounds.
    BottomRight,
    /// Top-left corner of content bounds.
    TopLeft,
    /// Top-right corner of content bounds.
    TopRight,
    /// Centered within content bounds.
    Center,
}

/// How an overlay is anchored relative to the chosen position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[serde(rename_all = "kebab-case")]
pub enum OverlayAnchorMode {
    /// Keep the overlay fully inside the content bounds.
    #[default]
    Inset,
    /// Center the overlay on the chosen content corner so it can hang off.
    Centered,
}

// ============================================================================
// ImageOverlayConfig
// ============================================================================

/// Configuration for image overlay — pure data.
///
/// Stores the image source and its [`Placement`]. Rendering logic
/// lives on [`Layer<ImageOverlayConfig>`].
///
/// # Image Sources
///
/// Accepts any [`ImageSource`]:
/// - SVG via [`ImageSource::svg()`] or [`ImageSource::from(SvgSource)`]
/// - Emoji via [`SvgSource::from_emoji()`] wrapped in [`ImageSource::Svg`]
/// - Raster images via [`ImageSource::raster()`]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ImageOverlayConfig {
    /// The image source (SVG, emoji, or raster).
    pub source: ImageSource,

    /// Where the overlay sits and how big it is, relative to the icon's content bounds.
    pub placement: Placement,
}

impl ImageOverlayConfig {
    /// Creates a new overlay config from any image source.
    pub fn new(source: impl Into<ImageSource>, placement: Placement) -> Self {
        Self {
            source: source.into(),
            placement,
        }
    }

    /// Creates a new overlay config from raw SVG markup.
    ///
    /// Convenience constructor that wraps `SvgSource::from_svg()` in an `ImageSource`.
    pub fn from_svg(svg: impl Into<String>, placement: Placement) -> Self {
        Self::new(ImageSource::from(SvgSource::from_svg(svg)), placement)
    }

    /// Creates a new overlay config from an emoji.
    ///
    /// Returns an error if the emoji is not supported by twemoji_assets.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji(emoji: &str, placement: Placement) -> Result<Self, RenderError> {
        Ok(Self::new(
            ImageSource::from(SvgSource::from_emoji(emoji)?),
            placement,
        ))
    }

    /// Creates a new overlay config from an emoji name (e.g., "duck").
    ///
    /// Returns an error if the name is not recognized by twemoji_assets.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji_name(name: &str, placement: Placement) -> Result<Self, RenderError> {
        Ok(Self::new(
            ImageSource::from(SvgSource::from_emoji_name(name)?),
            placement,
        ))
    }

    /// Creates a new overlay config from PNG-encoded raster data.
    pub fn from_raster(png_data: Vec<u8>, placement: Placement) -> Self {
        Self::new(ImageSource::raster(png_data), placement)
    }
}

impl LayerConfig for ImageOverlayConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self.source != other.source || self.placement.differs_from(&other.placement)
    }
}

// ============================================================================
// Layer Rendering
// ============================================================================

impl Layer<ImageOverlayConfig> {
    /// Render this overlay layer, returning a tile for compositing.
    ///
    /// Returns `None` if inactive. The tile is a transparent canvas with
    /// the image rendered at the configured position.
    pub fn render_tile(
        &mut self,
        ctx: &mut RenderContext,
        key: CacheKey,
        deps: DependencyVersion,
    ) -> Result<Option<&RgbaImage>, RenderError> {
        self.cached_tile(key, deps, |config| render_overlay(config, ctx))
    }
}

/// Renders an overlay image onto a transparent tile at the configured position.
fn render_overlay(
    config: &ImageOverlayConfig,
    ctx: &RenderContext,
) -> Result<RgbaImage, RenderError> {
    let bounds = ctx.image.content_bounds;
    let min_dim = bounds.width.min(bounds.height) as f32;
    let overlay_size = (min_dim * config.placement.scale) as u32;

    let width = ctx.image.data.width();
    let height = ctx.image.data.height();
    let mut tile = RgbaImage::new(width, height);

    if overlay_size == 0 {
        return Ok(tile);
    }

    let overlay_img = config.source.render_at_size(overlay_size)?;
    let (x, y) = pixel_position(&config.placement, &bounds, &overlay_img);
    composite_over(&mut tile, &overlay_img, x, y);

    Ok(tile)
}

/// Pixel origin of an already-rasterized `image` placed within `bounds`.
///
/// Shared with the color dot so both layers place identically.
pub(crate) fn pixel_position(
    placement: &Placement,
    bounds: &RectPx,
    image: &RgbaImage,
) -> (i32, i32) {
    let (x, y) = placement.origin(
        bounds.width as i32,
        bounds.height as i32,
        image.width() as i32,
        image.height() as i32,
    );
    (bounds.x as i32 + x, bounds.y as i32 + y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_anchor_can_hang_off_bottom_right_corner() {
        let bounds = RectPx::new(10, 20, 100, 80);
        let image = RgbaImage::new(20, 10);
        let placement = Placement::new(
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Centered,
            0.5,
        );

        assert_eq!(pixel_position(&placement, &bounds, &image), (100, 95));
    }

    #[test]
    fn inset_anchor_keeps_bottom_right_inside_bounds() {
        let bounds = RectPx::new(10, 20, 100, 80);
        let image = RgbaImage::new(20, 10);
        let placement =
            Placement::new(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.5);

        assert_eq!(pixel_position(&placement, &bounds, &image), (90, 90));
    }

    #[test]
    fn overlay_config_without_anchor_mode_defaults_to_inset() {
        let config: ImageOverlayConfig = serde_json::from_str(
            r#"{"source":{"svg":{"raw":"<svg></svg>"}},"placement":{"position":"bottom-right","scale":0.5}}"#,
        )
        .unwrap();

        assert_eq!(config.placement.anchor_mode, OverlayAnchorMode::Inset);
    }
}
