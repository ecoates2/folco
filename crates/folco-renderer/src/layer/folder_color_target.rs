//! Color target layer configuration and application.
//!
//! Implements GIMP-style Hue/Saturation adjustment internally: hue is shifted by a
//! delta in degrees, while saturation and lightness are scaled by a
//! multiplicative factor derived from the delta value.
//!
//! The public API accepts RGB target colors. HSL conversion is handled
//! internally as part of the rendering logic.
//!
//! # GIMP Hue-Saturation algorithm
//!
//! For each pixel the operation converts to HSL, then:
//!
//! - **Hue**: `new_hue = old_hue + hue_shift`
//! - **Saturation**: `new_saturation = old_saturation * (1.0 + saturation_delta)`
//! - **Lightness**: `new_lightness = old_lightness * (1.0 + lightness_delta)`
//!
//! Where `saturation_delta` and `lightness_delta` are in the range \[-1.0, 1.0\].
//!
//! A delta of 0.0 leaves the channel unchanged, +1.0 doubles it,
//! and -1.0 drives it to zero.

use super::{CacheKey, CachedOutput, DependencyVersion, DominantColor, Layer, LayerConfig, LayerVersions, RenderContext};
use crate::error::RenderError;
use crate::icon::{IconImage, SurfaceColor};
use palette::{Hsl, IntoColor, Srgb};

// ============================================================================
// FolderColorTargetConfig
// ============================================================================

/// Configuration for color targeting — pure data.
///
/// Stores only the target RGB color. HSL delta computation and pixel
/// transformation are handled by [`Layer<FolderColorTargetConfig>::apply()`].
///
/// # Emitted Properties
///
/// When applied, the layer emits [`DominantColor`] from the target values.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct FolderColorTargetConfig {
    /// Target red channel (0–255).
    pub target_r: u8,
    /// Target green channel (0–255).
    pub target_g: u8,
    /// Target blue channel (0–255).
    pub target_b: u8,
}

impl FolderColorTargetConfig {
    /// Creates a new color target config from target RGB values.
    pub fn new(target_r: u8, target_g: u8, target_b: u8) -> Self {
        Self {
            target_r,
            target_g,
            target_b,
        }
    }
}

impl LayerConfig for FolderColorTargetConfig {
    fn differs_from(&self, other: &Self) -> bool {
        self.target_r != other.target_r
            || self.target_g != other.target_g
            || self.target_b != other.target_b
    }
}

// ============================================================================
// Layer Rendering
// ============================================================================

impl Layer<FolderColorTargetConfig> {
    /// Apply the color target layer to the render context, using cache if valid.
    ///
    /// Transforms `ctx.image` using GIMP-style HSL adjustment and emits
    /// [`DominantColor`] for downstream layers. If inactive, the context
    /// passes through unchanged.
    ///
    /// The [`SurfaceColor`] must be present in the render context.
    ///
    /// # Errors
    ///
    /// Returns an error if the transform fails.
    pub fn apply(
        &mut self,
        ctx: &mut RenderContext,
        key: CacheKey,
        _versions: &LayerVersions,
    ) -> Result<(), RenderError> {
        if !self.is_active() {
            return Ok(());
        }

        let deps = DependencyVersion::NONE; // Root layer — no upstream dependencies

        // Check cache first
        if let Some(CachedOutput::Image(img)) = self.get_cached(key, deps) {
            ctx.image = img.clone();
            let config = self.config().unwrap();
            ctx.set(DominantColor::new(config.target_r, config.target_g, config.target_b, 255));
            return Ok(());
        }

        let config = self.config().unwrap();
        let surface = ctx
            .get::<SurfaceColor>()
            .expect("SurfaceColor must be set in RenderContext");

        ctx.image = apply_folder_color_target(&ctx.image, surface, config);
        ctx.set(DominantColor::new(config.target_r, config.target_g, config.target_b, 255));

        // Cache the transformed image
        self.store(key, CachedOutput::Image(ctx.image.clone()), deps);
        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Applies GIMP-style HSL color targeting to an icon image.
///
/// Computes hue/saturation/lightness deltas from `surface_color` and
/// `config.target_r/g/b`, then for each opaque pixel:
///
/// 1. Converts sRGB → HSL
/// 2. Shifts hue by `target_hue − surface_hue`
/// 3. Scales saturation by `target_sat / surface_sat`
/// 4. Scales lightness by `target_light / surface_light`
/// 5. Clamps S and L to \[0.0, 1.0\]
/// 6. Converts back to sRGB
pub(crate) fn apply_folder_color_target(
    icon: &IconImage,
    surface: &SurfaceColor,
    config: &FolderColorTargetConfig,
) -> IconImage {
    // Compute HSL deltas from surface → target
    let surface_rgb = Srgb::new(
        surface.r as f32 / 255.0,
        surface.g as f32 / 255.0,
        surface.b as f32 / 255.0,
    );
    let surface_hsl: Hsl = surface_rgb.into_color();
    let surface_hue = surface_hsl.hue.into_positive_degrees();
    let surface_saturation = surface_hsl.saturation;
    let surface_lightness = surface_hsl.lightness;

    let target_rgb = Srgb::new(
        config.target_r as f32 / 255.0,
        config.target_g as f32 / 255.0,
        config.target_b as f32 / 255.0,
    );
    let target_hsl: Hsl = target_rgb.into_color();
    let target_hue = target_hsl.hue.into_positive_degrees();
    let target_saturation = target_hsl.saturation;
    let target_lightness = target_hsl.lightness;

    let hue_shift = (target_hue - surface_hue).rem_euclid(360.0);
    let sat_factor = if surface_saturation > 0.0 {
        (target_saturation / surface_saturation).clamp(0.0, 2.0)
    } else {
        1.0
    };
    let light_factor = if surface_lightness > 0.0 {
        (target_lightness / surface_lightness).clamp(0.0, 2.0)
    } else {
        1.0
    };

    // Apply per-pixel
    let mut result = icon.data.clone();
    for pixel in result.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        if a == 0 {
            continue;
        }

        let rgb = Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let mut hsl: Hsl = rgb.into_color();

        hsl.hue += hue_shift;
        hsl.saturation = (hsl.saturation * sat_factor).clamp(0.0, 1.0);
        hsl.lightness = (hsl.lightness * light_factor).clamp(0.0, 1.0);

        let mutated: Srgb = hsl.into_color();
        pixel.0 = [
            (mutated.red * 255.0).round() as u8,
            (mutated.green * 255.0).round() as u8,
            (mutated.blue * 255.0).round() as u8,
            a,
        ];
    }

    IconImage::new(result, icon.scale, icon.content_bounds)
}

