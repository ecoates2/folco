//! Image overlay layer — configuration and rendering.
//!
//! Supports both vector (SVG/emoji) and raster (PNG) overlay sources
//! via [`ImageSource`].

use super::image_source::ImageSource;
use super::svg::{composite_over, SvgSource};
use super::{CacheKey, CachedOutput, DependencyVersion, Layer, LayerConfig, LayerVersions, RenderContext};
use crate::error::RenderError;
use image::RgbaImage;

// ============================================================================
// OverlayPosition
// ============================================================================

/// Position for image overlay placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
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
/// Stores the image source, position, and scale. Rendering logic
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

    /// Position within the icon's content bounds.
    pub position: OverlayPosition,

    /// How the overlay is anchored relative to the chosen position.
    #[serde(default)]
    pub anchor_mode: OverlayAnchorMode,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    pub scale: f32,
}

impl ImageOverlayConfig {
    /// Creates a new overlay config from any image source.
    ///
    /// The scale is clamped to 0.0-1.0.
    pub fn new(
        source: impl Into<ImageSource>,
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        scale: f32,
    ) -> Self {
        Self {
            source: source.into(),
            position,
            anchor_mode,
            scale: scale.clamp(0.0, 1.0),
        }
    }

    /// Creates a new overlay config from raw SVG markup.
    ///
    /// Convenience constructor that wraps `SvgSource::from_svg()` in an `ImageSource`.
    pub fn from_svg(
        svg: impl Into<String>,
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        scale: f32,
    ) -> Self {
        Self::new(
            ImageSource::from(SvgSource::from_svg(svg)),
            position,
            anchor_mode,
            scale,
        )
    }

    /// Creates a new overlay config from an emoji.
    ///
    /// Returns an error if the emoji is not supported by twemoji_assets.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji(
        emoji: &str,
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        scale: f32,
    ) -> Result<Self, RenderError> {
        Ok(Self {
            source: ImageSource::from(SvgSource::from_emoji(emoji)?),
            position,
            anchor_mode,
            scale: scale.clamp(0.0, 1.0),
        })
    }

    /// Creates a new overlay config from an emoji name (e.g., "duck").
    ///
    /// Returns an error if the name is not recognized by twemoji_assets.
    #[cfg(feature = "twemoji")]
    pub fn from_emoji_name(
        name: &str,
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        scale: f32,
    ) -> Result<Self, RenderError> {
        Ok(Self {
            source: ImageSource::from(SvgSource::from_emoji_name(name)?),
            position,
            anchor_mode,
            scale: scale.clamp(0.0, 1.0),
        })
    }

    /// Creates a new overlay config from PNG-encoded raster data.
    pub fn from_raster(
        png_data: Vec<u8>,
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        scale: f32,
    ) -> Self {
        Self::new(ImageSource::raster(png_data), position, anchor_mode, scale)
    }
}

impl LayerConfig for ImageOverlayConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self.source != other.source
            || self.position != other.position
            || self.anchor_mode != other.anchor_mode
            || (self.scale - other.scale).abs() > 0.0001
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
        _versions: &LayerVersions,
    ) -> Result<Option<RgbaImage>, RenderError> {
        if !self.is_active() {
            return Ok(None);
        }

        let deps = DependencyVersion::NONE; // No upstream dependencies

        if let Some(CachedOutput::Tile(tile)) = self.get_cached(key, deps) {
            return Ok(Some(tile.clone()));
        }

        let config = self.config().unwrap();
        let tile = render_overlay(config, ctx)?;

        self.store(key, CachedOutput::Tile(tile.clone()), deps);
        Ok(Some(tile))
    }
}

/// Renders an overlay image onto a transparent tile at the configured position.
fn render_overlay(
    config: &ImageOverlayConfig,
    ctx: &RenderContext,
) -> Result<RgbaImage, RenderError> {
    let bounds = ctx.image.content_bounds;
    let min_dim = bounds.width.min(bounds.height) as f32;
    let overlay_size = (min_dim * config.scale) as u32;

    let width = ctx.image.data.width();
    let height = ctx.image.data.height();
    let mut tile = RgbaImage::new(width, height);

    if overlay_size == 0 {
        return Ok(tile);
    }

    let overlay_img = config.source.render_at_size(overlay_size)?;

    let (x, y) = calculate_position(
        config.position,
        config.anchor_mode,
        &bounds,
        overlay_img.width(),
        overlay_img.height(),
    );

    composite_over(&mut tile, &overlay_img, x, y);

    Ok(tile)
}

/// Calculates the (x, y) position for the overlay based on position setting and bounds.
fn calculate_position(
    position: OverlayPosition,
    anchor_mode: OverlayAnchorMode,
    bounds: &crate::icon::RectPx,
    overlay_width: u32,
    overlay_height: u32,
) -> (i32, i32) {
    let bx = bounds.x as i32;
    let by = bounds.y as i32;
    let bw = bounds.width as i32;
    let bh = bounds.height as i32;
    let ow = overlay_width as i32;
    let oh = overlay_height as i32;

    match (position, anchor_mode) {
        (OverlayPosition::TopLeft, OverlayAnchorMode::Inset) => (bx, by),
        (OverlayPosition::TopRight, OverlayAnchorMode::Inset) => (bx + bw - ow, by),
        (OverlayPosition::BottomLeft, OverlayAnchorMode::Inset) => (bx, by + bh - oh),
        (OverlayPosition::BottomRight, OverlayAnchorMode::Inset) => (bx + bw - ow, by + bh - oh),
        (OverlayPosition::TopLeft, OverlayAnchorMode::Centered) => (bx - ow / 2, by - oh / 2),
        (OverlayPosition::TopRight, OverlayAnchorMode::Centered) => (bx + bw - ow / 2, by - oh / 2),
        (OverlayPosition::BottomLeft, OverlayAnchorMode::Centered) => (bx - ow / 2, by + bh - oh / 2),
        (OverlayPosition::BottomRight, OverlayAnchorMode::Centered) => (bx + bw - ow / 2, by + bh - oh / 2),
        (OverlayPosition::Center, _) => (bx + (bw - ow) / 2, by + (bh - oh) / 2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::RectPx;

    #[test]
    fn centered_anchor_can_hang_off_bottom_right_corner() {
        let bounds = RectPx::new(10, 20, 100, 80);

        let pos = calculate_position(
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Centered,
            &bounds,
            20,
            10,
        );

        assert_eq!(pos, (100, 95));
    }

    #[test]
    fn inset_anchor_keeps_bottom_right_inside_bounds() {
        let bounds = RectPx::new(10, 20, 100, 80);

        let pos = calculate_position(
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Inset,
            &bounds,
            20,
            10,
        );

        assert_eq!(pos, (90, 90));
    }

    #[test]
    fn overlay_config_without_anchor_mode_defaults_to_inset() {
        let config: ImageOverlayConfig = serde_json::from_str(
            r#"{"source":{"svg":{"raw":"<svg></svg>"}},"position":"bottom-right","scale":0.5}"#,
        )
        .unwrap();

        assert_eq!(config.anchor_mode, OverlayAnchorMode::Inset);
    }
}
