//! Decal imprint layer — configuration and rendering.

use super::svg::{SvgSource, render_source_with_color};
use super::{LayerConfig, RasterPaint, RenderContext};
use crate::error::RenderError;
use palette::{Hsl, IntoColor, Srgb};

const DECAL_DARKEN_AMOUNT: f32 = 0.25;

// ============================================================================
// DecalConfig
// ============================================================================

/// Configuration for decal imprint — pure data.
///
/// Stores the SVG source and scale factor. Rendering (color derivation,
/// positioning, compositing) happens in [`RasterPaint::paint`].
///
/// For full-color SVGs or emojis, use [`ImageOverlayConfig`](super::ImageOverlayConfig) instead.
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

impl RasterPaint for DecalConfig {
    /// Imprints the decal at the center of the content bounds.
    ///
    /// Tinted with a darkened [`RenderContext::dominant_color`] when a
    /// recolor ran upstream, otherwise with the darkened surface color.
    fn paint(&self, ctx: &mut RenderContext) -> Result<(), RenderError> {
        let dominant_color = ctx
            .dominant_color
            .map(|c| c.as_tuple())
            .unwrap_or_else(|| {
                let sc = ctx
                    .surface_color
                    .expect("SurfaceColor must be set in RenderContext");
                (sc.r, sc.g, sc.b, 255)
            });

        let darkened = darken_color(dominant_color, DECAL_DARKEN_AMOUNT);

        let bounds = ctx.image.content_bounds;
        let min_dim = bounds.width.min(bounds.height) as f32;
        let decal_size = (min_dim * self.scale) as u32;
        if decal_size == 0 {
            return Ok(());
        }

        let decal_img = render_source_with_color(&self.source, decal_size, Some(darkened))?;

        let center_x = bounds.x as i32 + (bounds.width as i32 - decal_img.width() as i32) / 2;
        let center_y = bounds.y as i32 + (bounds.height as i32 - decal_img.height() as i32) / 2;

        image::imageops::overlay(
            &mut ctx.image.data,
            &decal_img,
            i64::from(center_x),
            i64::from(center_y),
        );
        Ok(())
    }
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
