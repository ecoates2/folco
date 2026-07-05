//! Decal imprint layer — configuration and rendering.

use super::svg::{composite_over, render_source_with_color, SvgSource};
use super::{CacheKey, CachedOutput, DependencyVersion, DominantColor, Layer, LayerConfig, LayerVersions, RenderContext};
use crate::error::RenderError;
use crate::icon::SurfaceColor;
use image::RgbaImage;
use palette::{Hsl, IntoColor, Srgb};

const DECAL_DARKEN_AMOUNT: f32 = 0.25;

// ============================================================================
// DecalConfig
// ============================================================================

/// Configuration for decal imprint — pure data.
///
/// Stores the SVG source and scale factor. Rendering logic
/// (color derivation, positioning, compositing) lives on
/// [`Layer<DecalConfig>`].
///
/// For full-color SVGs or emojis, use [`ImageOverlayConfig`] instead.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DecalConfig {
    /// The SVG source (should be a monochrome/single-color SVG).
    pub source: SvgSource,

    /// Scale factor relative to the icon's content bounds (0.0-1.0).
    pub scale: f32,
}

impl DecalConfig {
    /// Creates a new decal config from an SVG string.
    ///
    /// The scale is clamped to 0.0-1.0.
    pub fn new(svg: impl Into<String>, scale: f32) -> Self {
        Self {
            source: SvgSource::Raw(svg.into()),
            scale: scale.clamp(0.0, 1.0),
        }
    }
}

impl LayerConfig for DecalConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self.source != other.source || (self.scale - other.scale).abs() > 0.0001
    }
}

// ============================================================================
// Layer Rendering
// ============================================================================

impl Layer<DecalConfig> {
    /// Render this decal layer, returning a tile for compositing.
    ///
    /// Returns `None` if inactive. The tile is a transparent canvas with
    /// the decal rendered at the center using a darkened version of the
    /// upstream [`DominantColor`] (or the [`SurfaceColor`] fallback).
    pub fn render_tile(
        &mut self,
        ctx: &mut RenderContext,
        key: CacheKey,
        versions: &LayerVersions,
    ) -> Result<Option<RgbaImage>, RenderError> {
        if !self.is_active() {
            return Ok(None);
        }

        let deps = DependencyVersion::from_version(versions.folder_color_target);

        if let Some(CachedOutput::Tile(tile)) = self.get_cached(key, deps) {
            return Ok(Some(tile.clone()));
        }

        let config = self.config().unwrap();
        let tile = render_decal(config, ctx)?;

        self.store(key, CachedOutput::Tile(tile.clone()), deps);
        Ok(Some(tile))
    }
}

/// Renders a decal onto a transparent tile matching the icon dimensions.
///
/// Uses [`DominantColor`] from the context if available, otherwise falls
/// back to the [`SurfaceColor`]. The color is darkened before rendering.
pub(crate) fn render_decal(
    config: &DecalConfig,
    ctx: &RenderContext,
) -> Result<RgbaImage, RenderError> {
    let dominant_color = ctx
        .get::<DominantColor>()
        .map(|c| c.as_tuple())
        .unwrap_or_else(|| {
            let sc = ctx
                .get::<SurfaceColor>()
                .expect("SurfaceColor must be set in RenderContext");
            (sc.r, sc.g, sc.b, 255)
        });

    let darkened = darken_color(dominant_color, DECAL_DARKEN_AMOUNT);

    let bounds = ctx.image.content_bounds;
    let min_dim = bounds.width.min(bounds.height) as f32;
    let decal_size = (min_dim * config.scale) as u32;

    let width = ctx.image.data.width();
    let height = ctx.image.data.height();
    let mut tile = RgbaImage::new(width, height);

    if decal_size == 0 {
        return Ok(tile);
    }

    let decal_img = render_source_with_color(&config.source, decal_size, Some(darkened))?;

    let center_x = bounds.x as i32 + (bounds.width as i32 - decal_img.width() as i32) / 2;
    let center_y = bounds.y as i32 + (bounds.height as i32 - decal_img.height() as i32) / 2;

    composite_over(&mut tile, &decal_img, center_x, center_y);

    Ok(tile)
}

// ============================================================================
// Color Utilities
// ============================================================================

/// Darkens an RGBA color by reducing its lightness.
pub fn darken_color(color: (u8, u8, u8, u8), amount: f32) -> (u8, u8, u8, u8) {
    let (r, g, b, a) = color;
    let rgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let mut hsl: Hsl = rgb.into_color();
    hsl.lightness = (hsl.lightness - amount).max(0.0);
    let darkened: Srgb = hsl.into_color();
    (
        (darkened.red * 255.0).round() as u8,
        (darkened.green * 255.0).round() as u8,
        (darkened.blue * 255.0).round() as u8,
        a,
    )
}
