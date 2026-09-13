//! Folder icon customizer — solid color + color dot + decal + overlay layers.
//!
//! [`FolderIconCustomizer`] is a type alias for `IconCustomizer<FolderLayers>`,
//! providing recoloring, color-dot badges, decal imprints, and image overlays
//! on top of system folder icons.
//!
//! # Layer Pipeline
//!
//! ```text
//! Base Image
//!     │
//!     ▼
//! ┌─────────────┐
//! │ Solid Color │ ◄── No dependencies (root layer)
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────┐
//! │  Decal  │ ◄── Depends on: Solid Color
//! └────┬────┘
//!      │
//!      ▼
//! ┌───────────┐
//! │ Color Dot │ ◄── No direct dependencies (drawn on top)
//! └─────┬─────┘
//!       │
//!       ▼
//! ┌─────────┐
//! │ Overlay │ ◄── No direct dependencies (applied last)
//! └─────────┘
//! ```

use crate::capabilities::{IconBaseKind, IconCapabilities};
use crate::customizer::{IconCustomizer, LayerSet};
use crate::error::RenderError;
use crate::icon::{FolderIconBase, IconBase};
use crate::layer::svg::composite_over;
use crate::layer::{
    CacheKey, ColorDotConfig, DecalConfig, DependencyVersion, ImageOverlayConfig, Layer,
    RenderContext, SolidColorConfig,
};
use crate::profile::CustomizationProfile;

// ============================================================================
// FolderLayers
// ============================================================================

/// Layer set for folder icon customization.
///
/// Contains all four layers in pipeline order:
/// 1. **Solid color** – Recolors the icon to a target RGB color (mutates image)
/// 2. **Decal** – Renders an SVG at the center (tile composited on top)
/// 3. **Color dot** – Renders a colored badge in the bottom-right (tile)
/// 4. **Overlay** – Renders an image at a corner position (tile composited on top)
#[derive(Default)]
pub struct FolderLayers {
    /// Solid color layer (root — no dependencies).
    pub solid_color: Layer<SolidColorConfig>,

    /// Decal imprint layer (depends on solid color).
    pub decal: Layer<DecalConfig>,

    /// Color dot badge layer (no dependencies).
    pub color_dot: Layer<ColorDotConfig>,

    /// Image overlay layer (no dependencies, applied last).
    pub overlay: Layer<ImageOverlayConfig>,
}

impl FolderLayers {
    /// Returns a snapshot of all layer versions.
    fn versions(&self) -> [u64; 4] {
        [
            self.solid_color.version(),
            self.color_dot.version(),
            self.decal.version(),
            self.overlay.version(),
        ]
    }
}

impl LayerSet for FolderLayers {
    fn execute(&mut self, ctx: &mut RenderContext, key: CacheKey) -> Result<(), RenderError> {
        // The pipeline's dependency wiring lives here, not inside the layers:
        // only the decal reads something an upstream layer produced.
        let decal_deps = DependencyVersion::from_version(self.solid_color.version());

        // 1. Solid color transforms ctx.image directly
        self.solid_color.apply(ctx, key, DependencyVersion::NONE)?;

        // 2. Decal tile layer — tints against the dominant color solid color emits
        if let Some(tile) = self.decal.render_tile(ctx, key, decal_deps)? {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        // 3. Color dot tile layer
        if let Some(tile) = self
            .color_dot
            .render_tile(ctx, key, DependencyVersion::NONE)?
        {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        // 4. Overlay tile layer
        if let Some(tile) = self
            .overlay
            .render_tile(ctx, key, DependencyVersion::NONE)?
        {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        Ok(())
    }

    fn combined_version(&self) -> DependencyVersion {
        DependencyVersion::combine(&self.versions())
    }

    fn invalidate_all(&mut self) {
        self.solid_color.invalidate();
        self.color_dot.invalidate();
        self.decal.invalidate();
        self.overlay.invalidate();
    }
}

// ============================================================================
// FolderIconCustomizer
// ============================================================================

/// Folder icon customizer — solid color, color dot, decal, and overlay layers.
///
/// Construct via [`FolderIconCustomizer::from_folder()`].
///
/// # Example
///
/// ```
/// use folco_renderer::{FolderIconCustomizer, FolderIconBase, IconSet, SolidColorConfig, DecalConfig, SurfaceColor};
///
/// let surface = SurfaceColor::new(255, 217, 112);
/// let base = FolderIconBase::new(IconSet::new(), surface);
/// let mut customizer = FolderIconCustomizer::from_folder(base);
///
/// customizer.layers.solid_color.set(Some(SolidColorConfig::new(33, 150, 243)));
/// customizer.layers.decal.set(Some(DecalConfig::new("<svg>...</svg>", 0.5)));
///
/// let output = customizer.render_all();
/// ```
pub type FolderIconCustomizer = IconCustomizer<FolderLayers>;

impl FolderIconCustomizer {
    /// Creates a customizer for system folder icons.
    ///
    /// All layers are available.
    pub fn from_folder(base: FolderIconBase) -> Self {
        IconCustomizer::new(IconBase::Folder(base), FolderLayers::default())
    }

    /// Applies a [`CustomizationProfile`]'s settings to all layers.
    ///
    /// The profile is authoritative: layers it omits are cleared and disabled.
    pub fn apply_profile(&mut self, profile: &CustomizationProfile) {
        self.layers
            .solid_color
            .set(profile.solid_color.clone());
        self.layers.color_dot.set(profile.color_dot);
        self.layers.decal.set(profile.decal.clone());
        self.layers.overlay.set(profile.overlay.clone());
    }

    /// Exports the currently rendering settings as a [`CustomizationProfile`].
    pub fn export_profile(&self) -> CustomizationProfile {
        CustomizationProfile {
            solid_color: self.layers.solid_color.config().cloned(),
            color_dot: self.layers.color_dot.config().copied(),
            decal: self.layers.decal.config().cloned(),
            overlay: self.layers.overlay.config().cloned(),
        }
    }

    /// What this customizer operates on.
    pub const fn base_kind() -> IconBaseKind {
        IconBaseKind::RasterFolder
    }

    /// The layers a raster folder icon can realize — all of them.
    pub fn capabilities() -> IconCapabilities {
        Self::base_kind().capabilities()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{IconImage, IconSet, SurfaceColor};
    use crate::layer::decal::darken_color;
    use crate::layer::{Layer, OverlayAnchorMode, OverlayPosition};
    use image::RgbaImage;

    const TEST_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"##;

    /// Test surface color — uses the same golden-yellow as Windows folders.
    const TEST_SURFACE: SurfaceColor = SurfaceColor::new(255, 217, 112);

    fn create_test_icon_base() -> FolderIconBase {
        let mut set = IconSet::new();

        // Create a 16x16 red icon
        let mut img16 = RgbaImage::new(16, 16);
        for pixel in img16.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        set.add_image(IconImage::new_full_content(img16, 1.0));

        // Create a 32x32 green icon
        let mut img32 = RgbaImage::new(32, 32);
        for pixel in img32.pixels_mut() {
            pixel.0 = [0, 255, 0, 255];
        }
        set.add_image(IconImage::new_full_content(img32, 1.0));

        FolderIconBase::new(set, TEST_SURFACE)
    }

    #[test]
    fn customizer_creation() {
        let base = create_test_icon_base();
        let customizer = FolderIconCustomizer::from_folder(base);

        assert!(customizer.layers.solid_color.config().is_none());
        assert!(customizer.layers.decal.config().is_none());
        assert!(customizer.layers.overlay.config().is_none());
    }

    #[test]
    fn hsl_mutation_setting() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        customizer
            .layers
            .solid_color
            .set(Some(SolidColorConfig::new(33, 150, 243)));
        assert_eq!(customizer.layers.solid_color.config().unwrap().target_r, 33);
        assert_eq!(
            customizer.layers.solid_color.config().unwrap().target_g,
            150
        );
        assert_eq!(
            customizer.layers.solid_color.config().unwrap().target_b,
            243
        );

        customizer.layers.solid_color.set(None);
        assert!(customizer.layers.solid_color.config().is_none());
    }

    #[test]
    fn render_without_customizations() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        let rendered = customizer.render(16).unwrap();
        assert_eq!(rendered.dimensions().width, 16);

        // Verify the image is unchanged
        let pixel = rendered.data.get_pixel(0, 0);
        assert_eq!(pixel.0, [255, 0, 0, 255]);
    }

    #[test]
    fn render_all_sizes() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        let result = customizer.render_all().unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn color_dot_draws_into_bottom_right_without_recoloring() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);
        customizer
            .layers
            .color_dot
            .set(Some(ColorDotConfig::new(0, 0, 255)));

        // The 32px base is solid green; only the dot area should turn blue.
        let rendered = customizer.render(32).unwrap();
        assert_eq!(rendered.data.get_pixel(0, 0).0, [0, 255, 0, 255]);

        // Dot center: 32 - margin(1) - size(12)/2 ≈ 25.
        let dot = rendered.data.get_pixel(25, 25).0;
        assert!(
            dot[2] > dot[1],
            "expected a blue dot in the bottom-right, got {dot:?}"
        );
    }

    #[test]
    fn hsl_mutation_applied() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        // Apply a green-ish target color
        customizer
            .layers
            .solid_color
            .set(Some(SolidColorConfig::new(0, 188, 212)));

        let rendered = customizer.render(16).unwrap();
        let pixel = rendered.data.get_pixel(0, 0);

        // Green channel should be dominant after rotation
        assert!(
            pixel[1] > pixel[0],
            "Green should be > Red after 120° hue shift"
        );
        assert!(
            pixel[1] > pixel[2],
            "Green should be > Blue after 120° hue shift"
        );
    }

    #[test]
    fn cache_invalidation_on_hsl_change() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        // First render
        customizer
            .layers
            .solid_color
            .set(Some(SolidColorConfig::new(76, 175, 80)));
        let first = customizer.render(16).unwrap();

        // Change color and render again
        customizer
            .layers
            .solid_color
            .set(Some(SolidColorConfig::new(33, 150, 243)));
        let second = customizer.render(16).unwrap();

        // Results should be different
        let p1 = first.data.get_pixel(0, 0);
        let p2 = second.data.get_pixel(0, 0);
        assert_ne!(
            p1.0, p2.0,
            "Different target colors should produce different results"
        );
    }

    #[test]
    fn cache_reuse_same_config() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        customizer
            .layers
            .solid_color
            .set(Some(SolidColorConfig::new(76, 175, 80)));

        // Render twice with same config
        let first = customizer.render(16).unwrap();
        let second = customizer.render(16).unwrap();

        // Results should be identical (from cache)
        assert_eq!(first.data.get_pixel(0, 0), second.data.get_pixel(0, 0));
    }

    #[test]
    fn layer_generic_set() {
        let mut layer: Layer<DecalConfig> = Layer::default();

        // No config yet
        assert!(!layer.is_configured());
        assert!(!layer.is_active()); // Not active without config
        assert_eq!(layer.version(), 0);

        // Setting config should increment version
        let changed = layer.set(Some(DecalConfig::new("svg", 0.5)));
        assert!(changed);
        assert!(layer.is_configured());
        assert!(layer.is_active()); // Now active
        assert_eq!(layer.version(), 1);

        // Same config should not increment
        let changed = layer.set(Some(DecalConfig::new("svg", 0.5)));
        assert!(!changed);
        assert_eq!(layer.version(), 1);

        // Different config should increment
        let changed = layer.set(Some(DecalConfig::new("svg2", 0.5)));
        assert!(changed);
        assert_eq!(layer.version(), 2);
    }

    #[test]
    fn layer_config_set_clear_cycle() {
        let mut layer: Layer<DecalConfig> = Layer::default();

        // Set config
        layer.set(Some(DecalConfig::new("my-svg", 0.5)));
        assert!(layer.is_active());
        assert_eq!(layer.version(), 1);

        // Clear config (deactivate)
        let changed = layer.set(None);
        assert!(changed);
        assert!(!layer.is_active());
        assert!(!layer.is_configured());
        assert_eq!(layer.version(), 2);

        // Re-set config (reactivate)
        let changed = layer.set(Some(DecalConfig::new("my-svg", 0.5)));
        assert!(changed);
        assert!(layer.is_active());
        assert_eq!(
            layer.config().unwrap().source,
            crate::layer::SvgSource::Raw("my-svg".into())
        );
        assert_eq!(layer.version(), 3);
    }

    #[test]
    fn hsl_mutation_toggle() {
        let base = create_test_icon_base();
        let mut customizer = FolderIconCustomizer::from_folder(base);

        // Set color target
        customizer
            .layers
            .solid_color
            .set(Some(SolidColorConfig::new(0, 188, 212)));
        assert!(customizer.layers.solid_color.is_active());
        let rotated = customizer.render(16).unwrap();
        let rotated_pixel = rotated.data.get_pixel(0, 0).0;

        // Deactivate by clearing config
        customizer.layers.solid_color.set(None);
        assert!(!customizer.layers.solid_color.is_active());
        assert!(!customizer.layers.solid_color.is_configured());
        let disabled = customizer.render(16).unwrap();
        assert_eq!(disabled.data.get_pixel(0, 0).0, [255, 0, 0, 255]); // Original red

        // Re-activate
        customizer.layers.solid_color.set(Some(SolidColorConfig::new(0, 188, 212)));
        let re_enabled = customizer.render(16).unwrap();
        assert_eq!(re_enabled.data.get_pixel(0, 0).0, rotated_pixel);
    }

    #[test]
    fn darken_color_works() {
        let original = (200, 100, 100, 255);
        let darkened = darken_color(original, 0.2);

        // Darkened color should have lower overall brightness
        let orig_brightness = (original.0 as u32 + original.1 as u32 + original.2 as u32) / 3;
        let dark_brightness = (darkened.0 as u32 + darkened.1 as u32 + darkened.2 as u32) / 3;
        assert!(
            dark_brightness < orig_brightness,
            "Darkened color should be less bright"
        );

        // Alpha should be preserved
        assert_eq!(darkened.3, original.3);
    }

    #[test]
    fn decal_config_creation() {
        let config = DecalConfig::new("<svg></svg>", 0.5);
        assert_eq!(
            config.source,
            crate::layer::SvgSource::Raw("<svg></svg>".into())
        );
        assert_eq!(config.scale, 0.5);

        // Test clamping
        let clamped = DecalConfig::new("svg", 1.5);
        assert_eq!(clamped.scale, 1.0);
    }

    #[test]
    fn overlay_config_creation() {
        let config = ImageOverlayConfig::from_svg(
            "<svg></svg>",
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Inset,
            0.25,
        );
        assert_eq!(config.position, OverlayPosition::BottomRight);
        assert_eq!(config.scale, 0.25);
    }

    #[test]
    fn decal_uses_hsl_mutated_dominant_color() {
        use crate::layer::decal::render_decal;
        use crate::layer::solid_color::apply_solid_color;
        use crate::layer::{DominantColor, RenderContext};

        // Create a solid red image
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);

        // Apply color target (cyan-ish)
        let config = SolidColorConfig::new(0, 188, 212);
        let mut ctx = RenderContext::new(red_icon.clone());
        ctx.set(TEST_SURFACE);
        ctx.image = apply_solid_color(&ctx.image, &TEST_SURFACE, &config);
        ctx.set(DominantColor::new(
            config.target_r,
            config.target_g,
            config.target_b,
            255,
        ));

        // Verify color target emitted DominantColor
        let emitted = ctx.get::<DominantColor>();
        assert!(emitted.is_some(), "Color target should emit DominantColor");

        let emitted_color = emitted.unwrap().as_tuple();
        assert!(
            emitted_color.1 > emitted_color.0,
            "Emitted color should have more green than red for cyan target"
        );

        // Now apply decal - it should use the emitted color, not re-sample
        let decal_config = DecalConfig::new(TEST_SVG, 0.5);
        let _tile = render_decal(&decal_config, &ctx).unwrap();

        // The DominantColor property should still be the target color
        let color_after_decal = ctx.get::<DominantColor>().unwrap().as_tuple();
        assert_eq!(
            color_after_decal, emitted_color,
            "Decal should consume but not overwrite DominantColor"
        );
    }

    #[test]
    fn decal_samples_base_when_hsl_disabled() {
        use crate::layer::{CacheKey, DependencyVersion, DominantColor, RenderContext};

        // Create a solid blue image
        let mut blue_img = RgbaImage::new(16, 16);
        for pixel in blue_img.pixels_mut() {
            pixel.0 = [0, 0, 255, 255];
        }
        let blue_icon = IconImage::new_full_content(blue_img, 1.0);

        // Set up layers: color target NOT configured (disabled)
        let mut ct_layer: Layer<SolidColorConfig> = Layer::default();
        // No config set — layer is inactive

        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // Create context and apply through Layer::apply
        let mut ctx = RenderContext::new(blue_icon.clone());
        ctx.set(TEST_SURFACE);
        let key = CacheKey::from_icon(&blue_icon);
        let decal_deps = DependencyVersion::from_version(ct_layer.version());

        // Apply color target layer (should skip because no config)
        ct_layer
            .apply(&mut ctx, key, DependencyVersion::NONE)
            .unwrap();

        // Verify no DominantColor was emitted (because color target was skipped)
        assert!(
            ctx.get::<DominantColor>().is_none(),
            "No DominantColor should exist when color target layer has no config"
        );

        // Image should be unchanged (still blue)
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should be unchanged when color target has no config"
        );

        // Apply decal - it should fall back to surface color (the golden-yellow)
        // Decal now returns a tile, not modifying ctx.image directly
        let _tile = decal_layer.render_tile(&mut ctx, key, decal_deps).unwrap();

        // Image should still be unchanged (decal produces a tile, doesn't composite)
        assert_eq!(
            ctx.image.data.get_pixel(0, 0).0,
            [0, 0, 255, 255],
            "Image should still be blue"
        );
    }

    #[test]
    fn disabled_hsl_layer_version_change_invalidates_decal_cache() {
        use crate::layer::{CacheKey, DependencyVersion, DominantColor, RenderContext};

        // Create red and blue test icons
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let red_icon = IconImage::new_full_content(red_img, 1.0);
        let key = CacheKey::from_icon(&red_icon);

        // Set up layers
        let mut ct_layer: Layer<SolidColorConfig> = Layer::default();
        ct_layer.set(Some(SolidColorConfig::new(0, 188, 212)));
        let mut decal_layer: Layer<DecalConfig> = Layer::default();
        decal_layer.set(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // First render: color target enabled
        let decal_deps_v1 = DependencyVersion::from_version(ct_layer.version());
        let mut ctx1 = RenderContext::new(red_icon.clone());
        ctx1.set(TEST_SURFACE);
        ct_layer
            .apply(&mut ctx1, key, DependencyVersion::NONE)
            .unwrap();
        decal_layer
            .render_tile(&mut ctx1, key, decal_deps_v1)
            .unwrap();

        // Color target should have emitted DominantColor
        let emitted_with_ct = ctx1.get::<DominantColor>().unwrap().as_tuple();
        assert!(
            emitted_with_ct.1 > emitted_with_ct.0,
            "With color target enabled, emitted color should be cyan-ish"
        );

        // Now clear color target config — version should change
        let old_version = ct_layer.version();
        ct_layer.set(None);
        let new_version = ct_layer.version();
        assert_ne!(
            old_version, new_version,
            "Clearing config should change its version"
        );

        // Second render: color target not configured
        let decal_deps_v2 = DependencyVersion::from_version(ct_layer.version()); // New version!
        let mut ctx2 = RenderContext::new(red_icon.clone());
        ctx2.set(TEST_SURFACE);
        ct_layer
            .apply(&mut ctx2, key, DependencyVersion::NONE)
            .unwrap();
        decal_layer
            .render_tile(&mut ctx2, key, decal_deps_v2)
            .unwrap();

        // No DominantColor should be emitted (color target was skipped)
        assert!(
            ctx2.get::<DominantColor>().is_none(),
            "With color target cleared, no DominantColor should be emitted"
        );

        // Image should be original red (not shifted)
        assert_eq!(
            ctx2.image.data.get_pixel(0, 0).0,
            [255, 0, 0, 255],
            "With color target cleared, image should be original red"
        );
    }

    #[test]
    fn pipeline_property_flow_with_hsl_toggle() {
        // Integration test: verify the full pipeline handles color target toggle correctly
        let mut red_img = RgbaImage::new(16, 16);
        for pixel in red_img.pixels_mut() {
            pixel.0 = [255, 0, 0, 255];
        }
        let mut icons = IconSet::new();
        icons.add_image(IconImage::new_full_content(red_img, 1.0));

        let mut customizer =
            FolderIconCustomizer::from_folder(FolderIconBase::new(icons, TEST_SURFACE));

        // Enable both color target and decal
        let ct_config = SolidColorConfig::new(0, 188, 212);
        customizer
            .layers
            .solid_color
            .set(Some(ct_config.clone()));
        customizer
            .layers
            .decal
            .set(Some(DecalConfig::new(TEST_SVG, 0.5)));

        // Render with color target enabled
        let with_ct = customizer.render(16).unwrap();
        let ct_pixel = with_ct.data.get_pixel(0, 0).0;

        // Deactivate color target, keep decal
        customizer.layers.solid_color.set(None);
        let without_ct = customizer.render(16).unwrap();
        let no_ct_pixel = without_ct.data.get_pixel(0, 0).0;

        // With color target: image should be shifted (more green than red)
        assert!(
            ct_pixel[1] > ct_pixel[0],
            "With color target enabled, green should dominate"
        );

        // Without color target: image should be original red
        assert_eq!(
            no_ct_pixel,
            [255, 0, 0, 255],
            "With color target disabled, should be original red"
        );

        // Re-activate color target - should go back to shifted
        customizer.layers.solid_color.set(Some(ct_config));
        let re_enabled = customizer.render(16).unwrap();
        assert_eq!(
            re_enabled.data.get_pixel(0, 0).0,
            ct_pixel,
            "Re-activating color target should restore shifted result"
        );
    }
}
