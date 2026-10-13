//! Color-dot layer — a small colored badge drawn on top of the icon.
//!
//! Unlike [`SolidColorConfig`](super::SolidColorConfig), which recolors the
//! whole icon and therefore needs a known surface color, the dot only draws on
//! top. That makes it usable on every base icon: system folders (raster or
//! vector) and user-supplied images alike.
//!
//! The dot is placed exactly like an [`ImageOverlayConfig`](super::ImageOverlayConfig):
//! the same position, anchor mode and scale semantics, sharing the same
//! placement math. By default it sits in the bottom-right corner.
//!
//! The dot's markup lives here so the raster and vector media stay pixel-for-
//! pixel equivalent: the raster layer rasterizes [`color_dot_svg`], while
//! [`SvgLayer<ColorDotConfig>`](crate::SvgLayer) nests the same circle markup
//! in the output document.

use super::overlay::calculate_position;
use super::svg::composite_over;
use super::{
    CacheKey, DependencyVersion, ImageSource, Layer, LayerConfig, OverlayAnchorMode,
    OverlayPosition, RenderContext,
};
use crate::error::RenderError;
use image::RgbaImage;

// ============================================================================
// Geometry
// ============================================================================

/// Default dot edge length as a fraction of the icon's shorter content dimension.
pub const DEFAULT_COLOR_DOT_SCALE: f32 = 0.40;

/// Default dot position within the icon's content bounds.
pub const DEFAULT_COLOR_DOT_POSITION: OverlayPosition = OverlayPosition::BottomRight;

/// The dot's own coordinate space, kept independent of the base icon's viewBox.
pub(crate) const DOT_VIEWBOX: &str = "0 0 100 100";

// ============================================================================
// ColorDotConfig
// ============================================================================

/// Configuration for the color dot — pure data.
///
/// Placement fields mirror [`ImageOverlayConfig`](super::ImageOverlayConfig)
/// and default to the bottom-right corner, so profiles that only carry
/// `r`/`g`/`b` keep deserializing.
///
/// Rendering logic lives on [`Layer<ColorDotConfig>`] (raster) and
/// [`SvgLayer<ColorDotConfig>`](crate::SvgLayer) (vector).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ColorDotConfig {
    /// Red channel (0–255).
    pub r: u8,
    /// Green channel (0–255).
    pub g: u8,
    /// Blue channel (0–255).
    pub b: u8,

    /// Position within the icon's content bounds.
    #[serde(default = "default_position")]
    pub position: OverlayPosition,

    /// How the dot is anchored relative to the chosen position.
    #[serde(default)]
    pub anchor_mode: OverlayAnchorMode,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    #[serde(default = "default_scale")]
    pub scale: f32,
}

fn default_position() -> OverlayPosition {
    DEFAULT_COLOR_DOT_POSITION
}

fn default_scale() -> f32 {
    DEFAULT_COLOR_DOT_SCALE
}

impl ColorDotConfig {
    /// Creates a new color-dot config from RGB values, placed in the
    /// bottom-right corner at the default scale.
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self {
            r,
            g,
            b,
            position: DEFAULT_COLOR_DOT_POSITION,
            anchor_mode: OverlayAnchorMode::default(),
            scale: DEFAULT_COLOR_DOT_SCALE,
        }
    }

    /// Returns this config with the given placement.
    ///
    /// The scale is clamped to 0.0-1.0.
    #[must_use]
    pub fn with_placement(
        mut self,
        position: OverlayPosition,
        anchor_mode: OverlayAnchorMode,
        scale: f32,
    ) -> Self {
        self.position = position;
        self.anchor_mode = anchor_mode;
        self.scale = scale.clamp(0.0, 1.0);
        self
    }
}

impl LayerConfig for ColorDotConfig {
    fn differs_from(&self, other: &Self) -> bool {
        (self.r, self.g, self.b) != (other.r, other.g, other.b)
            || self.position != other.position
            || self.anchor_mode != other.anchor_mode
            || (self.scale - other.scale).abs() > 0.0001
    }
}

// ============================================================================
// Shared markup
// ============================================================================

/// The dot itself, in [`DOT_VIEWBOX`] coordinates.
///
/// The light ring keeps the dot legible over dark folder art.
pub(crate) fn color_dot_circle(config: &ColorDotConfig) -> String {
    let fill = format!("#{:02x}{:02x}{:02x}", config.r, config.g, config.b);
    format!(
        "<circle cx=\"50\" cy=\"50\" r=\"42\" fill=\"{fill}\" \
stroke=\"#ffffff\" stroke-width=\"8\"/>"
    )
}

/// A standalone SVG document containing just the dot, for rasterization.
pub(crate) fn color_dot_svg(config: &ColorDotConfig) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{DOT_VIEWBOX}\">{}</svg>",
        color_dot_circle(config)
    )
}

// ============================================================================
// Layer Rendering
// ============================================================================

impl Layer<ColorDotConfig> {
    /// Renders the color dot to a transparent tile for compositing.
    ///
    /// Returns `None` if the layer is inactive.
    ///
    /// # Errors
    ///
    /// Returns an error if the dot cannot be rasterized.
    pub fn render_tile(
        &mut self,
        ctx: &RenderContext,
        key: CacheKey,
        deps: DependencyVersion,
    ) -> Result<Option<&RgbaImage>, RenderError> {
        self.cached_tile(key, deps, |config| render_color_dot(config, ctx))
    }
}

/// Rasterizes the dot at its configured position within the icon's content bounds.
fn render_color_dot(
    config: &ColorDotConfig,
    ctx: &RenderContext,
) -> Result<RgbaImage, RenderError> {
    let bounds = ctx.image.content_bounds;
    let mut tile = RgbaImage::new(ctx.image.data.width(), ctx.image.data.height());

    let min_dim = bounds.width.min(bounds.height) as f32;
    let dot_size = (min_dim * config.scale) as u32;
    if dot_size == 0 {
        return Ok(tile);
    }

    let dot = ImageSource::svg(color_dot_svg(config)).render_at_size(dot_size)?;

    let (x, y) = calculate_position(
        config.position,
        config.anchor_mode,
        &bounds,
        dot.width(),
        dot.height(),
    );
    composite_over(&mut tile, &dot, x, y);

    Ok(tile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{IconImage, RectPx};

    fn context(size: u32) -> RenderContext {
        RenderContext::new(IconImage::new(
            RgbaImage::new(size, size),
            1.0,
            RectPx::from_size(size, size),
        ))
    }

    #[test]
    fn inactive_layer_renders_nothing() {
        let mut layer: Layer<ColorDotConfig> = Layer::default();
        let ctx = context(64);
        let key = CacheKey::from_icon(&ctx.image);

        assert!(
            layer
                .render_tile(&ctx, key, DependencyVersion::NONE)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn active_layer_draws_into_bottom_right() {
        let mut layer: Layer<ColorDotConfig> = Layer::default();
        layer.set_config(Some(ColorDotConfig::new(33, 150, 243)));

        let ctx = context(64);
        let key = CacheKey::from_icon(&ctx.image);

        let tile = layer
            .render_tile(&ctx, key, DependencyVersion::NONE)
            .unwrap()
            .unwrap();

        // Dot center: 64 - size(25)/2 ≈ 51.
        let center = tile.get_pixel(51, 51);
        assert!(center[3] > 0, "dot center should be opaque");
        assert!(
            center[2] > center[0],
            "blue should dominate for #2196f3, got {center:?}"
        );

        // The top-left stays untouched so the tile composites cleanly.
        assert_eq!(tile.get_pixel(0, 0).0[3], 0);
    }

    #[test]
    fn placement_moves_and_scales_the_dot() {
        let mut layer: Layer<ColorDotConfig> = Layer::default();
        layer.set_config(Some(ColorDotConfig::new(255, 0, 0).with_placement(
            OverlayPosition::TopLeft,
            OverlayAnchorMode::Inset,
            0.5,
        )));

        let ctx = context(64);
        let key = CacheKey::from_icon(&ctx.image);

        let tile = layer
            .render_tile(&ctx, key, DependencyVersion::NONE)
            .unwrap()
            .unwrap();

        // 32px dot in the top-left: centered on (16, 16).
        assert!(tile.get_pixel(16, 16).0[3] > 0, "dot should be top-left");
        assert_eq!(
            tile.get_pixel(48, 48).0[3],
            0,
            "bottom-right should be empty"
        );
    }

    #[test]
    fn rgb_only_json_defaults_to_bottom_right() {
        let config: ColorDotConfig = serde_json::from_str(r#"{"r":1,"g":2,"b":3}"#).unwrap();
        assert_eq!(config, ColorDotConfig::new(1, 2, 3));
    }

    #[test]
    fn with_placement_clamps_scale() {
        let config = ColorDotConfig::new(0, 0, 0).with_placement(
            OverlayPosition::Center,
            OverlayAnchorMode::Inset,
            1.5,
        );
        assert_eq!(config.scale, 1.0);
    }

    #[test]
    fn differs_from_detects_color_change() {
        let a = ColorDotConfig::new(0, 0, 0);
        let b = ColorDotConfig::new(0, 0, 1);
        assert!(a.differs_from(&b));
        assert!(!a.differs_from(&a));
    }

    #[test]
    fn differs_from_detects_placement_change() {
        let a = ColorDotConfig::new(0, 0, 0);
        let b = a.with_placement(
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Centered,
            0.4,
        );
        let c = a.with_placement(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.3);
        assert!(a.differs_from(&b));
        assert!(a.differs_from(&c));
    }
}
