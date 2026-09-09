//! Color-dot layer — a small colored badge in the icon's bottom-right corner.
//!
//! Unlike [`SolidColorConfig`](super::SolidColorConfig), which recolors the
//! whole icon and therefore needs a known surface color, the dot only draws on
//! top. That makes it usable on every base icon: system folders (raster or
//! vector) and user-supplied images alike.
//!
//! The dot's geometry lives here so the raster and vector media stay pixel-for-
//! pixel equivalent: the raster layer rasterizes [`color_dot_svg`], while
//! [`SvgLayer<ColorDotConfig>`](crate::SvgLayer) nests the same circle markup
//! in the output document.

use super::svg::composite_over;
use super::{
    CacheKey, CachedOutput, DependencyVersion, ImageSource, Layer, LayerConfig, LayerVersions,
    RenderContext,
};
use crate::error::RenderError;
use image::RgbaImage;

// ============================================================================
// Geometry
// ============================================================================

/// Dot edge length as a fraction of the icon's shorter content dimension.
pub(crate) const DOT_SCALE: f32 = 0.40;

/// Gap between the dot and the bottom-right corner, in the same units.
pub(crate) const DOT_MARGIN: f32 = 0.05;

/// The dot's own coordinate space, kept independent of the base icon's viewBox.
pub(crate) const DOT_VIEWBOX: &str = "0 0 100 100";

// ============================================================================
// ColorDotConfig
// ============================================================================

/// Configuration for the color dot — pure data.
///
/// Rendering logic lives on [`Layer<ColorDotConfig>`] (raster) and
/// [`SvgLayer<ColorDotConfig>`](crate::SvgLayer) (vector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
        _versions: &LayerVersions,
    ) -> Result<Option<RgbaImage>, RenderError> {
        if !self.is_active() {
            return Ok(None);
        }

        let deps = DependencyVersion::NONE; // Draws on top; no upstream inputs.

        if let Some(CachedOutput::Tile(tile)) = self.get_cached(key, deps) {
            return Ok(Some(tile.clone()));
        }

        let config = self.config().expect("active layer always has a config");
        let tile = render_color_dot(config, ctx)?;

        self.store(key, CachedOutput::Tile(tile.clone()), deps);
        Ok(Some(tile))
    }
}

/// Rasterizes the dot into the bottom-right of the icon's content bounds.
fn render_color_dot(
    config: &ColorDotConfig,
    ctx: &RenderContext,
) -> Result<RgbaImage, RenderError> {
    let bounds = ctx.image.content_bounds;
    let mut tile = RgbaImage::new(ctx.image.data.width(), ctx.image.data.height());

    let min_dim = bounds.width.min(bounds.height) as f32;
    let dot_size = (min_dim * DOT_SCALE) as u32;
    if dot_size == 0 {
        return Ok(tile);
    }

    let dot = ImageSource::svg(color_dot_svg(config)).render_at_size(dot_size)?;

    let margin = (min_dim * DOT_MARGIN) as i32;
    let x = (bounds.x + bounds.width) as i32 - dot.width() as i32 - margin;
    let y = (bounds.y + bounds.height) as i32 - dot.height() as i32 - margin;
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
        let versions = LayerVersions {
            solid_color: 0,
            color_dot: 0,
            decal: 0,
            overlay: 0,
        };

        assert!(layer.render_tile(&ctx, key, &versions).unwrap().is_none());
    }

    #[test]
    fn active_layer_draws_into_bottom_right() {
        let mut layer: Layer<ColorDotConfig> = Layer::default();
        layer.set_config(Some(ColorDotConfig::new(33, 150, 243)));

        let ctx = context(64);
        let key = CacheKey::from_icon(&ctx.image);
        let versions = LayerVersions {
            solid_color: 0,
            color_dot: layer.version(),
            decal: 0,
            overlay: 0,
        };

        let tile = layer.render_tile(&ctx, key, &versions).unwrap().unwrap();

        // Dot center: 64 - margin(3) - size(25)/2 ≈ 48.
        let center = tile.get_pixel(48, 48);
        assert!(center[3] > 0, "dot center should be opaque");
        assert!(
            center[2] > center[0],
            "blue should dominate for #2196f3, got {center:?}"
        );

        // The top-left stays untouched so the tile composites cleanly.
        assert_eq!(tile.get_pixel(0, 0).0[3], 0);
    }

    #[test]
    fn differs_from_detects_color_change() {
        let a = ColorDotConfig::new(0, 0, 0);
        let b = ColorDotConfig::new(0, 0, 1);
        assert!(a.differs_from(&b));
        assert!(!a.differs_from(&a));
    }
}
