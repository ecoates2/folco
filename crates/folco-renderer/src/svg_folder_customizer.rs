//! SVG folder icon customizer.
//!
//! [`SvgFolderIconCustomizer`] is the vector-medium sibling of
//! [`FolderIconCustomizer`](crate::FolderIconCustomizer). Where the raster
//! customizer mutates pixels and emits an [`IconSet`](crate::IconSet), this
//! customizer appends SVG fragments to an [`SvgCanvas`] and emits a single
//! scalable SVG string.
//!
//! # Layers
//!
//! [`SvgFolderLayers`] holds SVG-compatible layers that realize medium-neutral
//! profile intents in vector form. Currently:
//! - **Color dot** — the vector analogue of raster color targeting. Because an
//!   arbitrary SVG can't be HSL-shifted, the folder-color intent is expressed
//!   as a small colored dot overlaid on the icon.
//! - **Overlay** — the vector analogue of the raster image overlay. The source
//!   (SVG, emoji, or raster) is embedded as an SVG `<image>` element.

use crate::error::RenderError;
use crate::icon::{IconImage, SurfaceColor, SvgFolderIconBase};
use crate::layer::{DependencyVersion, FolderColorTargetConfig, ImageOverlayConfig, ImageSource};
use crate::medium::SvgCanvas;
use crate::profile::CustomizationProfile;
use crate::svg_layer::{ColorDotConfig, SvgLayer};

// ============================================================================
// SvgLayerSet
// ============================================================================

/// Trait for an ordered set of SVG-medium layers.
///
/// The vector counterpart to [`LayerSet`](crate::LayerSet). Implementors append
/// markup to the [`SvgCanvas`] in order; the customizer serializes the result
/// once (SVG output is resolution-independent, so there is no per-size loop).
pub trait SvgLayerSet {
    /// Execute all layers against the canvas in order.
    fn execute(&mut self, canvas: &mut SvgCanvas) -> Result<(), RenderError>;

    /// Combined version of all layers, used to detect changes for caching.
    fn combined_version(&self) -> DependencyVersion;

    /// Invalidate all layer caches.
    fn invalidate_all(&mut self);
}

// ============================================================================
// SvgFolderLayers
// ============================================================================

/// Layer set for SVG folder customization.
///
/// Holds SVG-compatible layers, in composition order.
#[derive(Debug, Default)]
pub struct SvgFolderLayers {
    /// Color-dot overlay — the vector analogue of raster color targeting.
    pub color_dot: SvgLayer<ColorDotConfig>,

    /// Image overlay — an embedded SVG `<image>` (SVG/emoji/raster source).
    pub overlay: SvgLayer<ImageOverlayConfig>,
}

impl SvgLayerSet for SvgFolderLayers {
    fn execute(&mut self, canvas: &mut SvgCanvas) -> Result<(), RenderError> {
        if let Some(fragment) = self.color_dot.render_fragment() {
            canvas.push_overlay(fragment);
        }
        if let Some(fragment) = self.overlay.render_fragment()? {
            canvas.push_overlay(fragment);
        }
        Ok(())
    }

    fn combined_version(&self) -> DependencyVersion {
        DependencyVersion::combine(&[self.color_dot.version(), self.overlay.version()])
    }

    fn invalidate_all(&mut self) {
        self.color_dot.invalidate();
        self.overlay.invalidate();
    }
}

// ============================================================================
// SvgFolderIconCustomizer
// ============================================================================

/// SVG folder icon customizer — vector-medium sibling of
/// [`FolderIconCustomizer`](crate::FolderIconCustomizer).
///
/// Construct via [`SvgFolderIconCustomizer::from_folder()`]. Configure layers
/// through the [`layers`](Self::layers) field, then call
/// [`render`](Self::render) to produce a scalable SVG string.
///
/// # Example
///
/// ```
/// use folco_renderer::{SvgFolderIconBase, SvgFolderIconCustomizer, SurfaceColor};
///
/// let base = SvgFolderIconBase::new(
///     r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"></svg>"#,
///     SurfaceColor::new(255, 217, 112),
/// );
/// let mut customizer = SvgFolderIconCustomizer::from_folder(base);
///
/// // With no layers configured, rendering returns the base markup unchanged.
/// let svg = customizer.render_output().unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub struct SvgFolderIconCustomizer {
    base: SvgFolderIconBase,

    /// The SVG layer set. Access layers directly to configure them.
    pub layers: SvgFolderLayers,
}

impl SvgFolderIconCustomizer {
    /// Creates a customizer for a scalable system folder icon.
    pub fn from_folder(base: SvgFolderIconBase) -> Self {
        Self {
            base,
            layers: SvgFolderLayers::default(),
        }
    }

    /// Returns a reference to the SVG folder base.
    pub fn base(&self) -> &SvgFolderIconBase {
        &self.base
    }

    /// Returns the surface color reference.
    pub fn surface_color(&self) -> &SurfaceColor {
        &self.base.surface_color
    }

    /// Renders the artifact written to the system: a single scalable SVG string.
    ///
    /// With no active layers this returns the base markup unchanged.
    ///
    /// # Errors
    ///
    /// Returns a render error if a layer fails.
    pub fn render_output(&mut self) -> Result<String, RenderError> {
        let mut canvas = SvgCanvas::new(&self.base.svg);
        self.layers.execute(&mut canvas)?;
        Ok(canvas.into_svg())
    }

    /// Renders a rasterized preview at `size` px for on-screen display.
    ///
    /// Previews are pixels so both media can share one display path; the
    /// artifact written to the system still comes from [`render_output`](Self::render_output).
    ///
    /// # Errors
    ///
    /// Returns a render error if a layer fails or the SVG cannot be rasterized.
    pub fn render_preview(&mut self, size: u32) -> Result<IconImage, RenderError> {
        let svg = self.render_output()?;
        let rgba = ImageSource::svg(svg).render_at_size(size)?;
        Ok(IconImage::new_full_content(rgba, 1.0))
    }

    /// Applies a [`CustomizationProfile`]'s settings to the SVG layers.
    ///
    /// Medium-neutral intents are realized in vector form: the folder-color
    /// intent (`folder_color_target`) becomes a color-dot overlay rather than
    /// an HSL shift, and the `overlay` intent becomes an embedded `<image>`.
    pub fn apply_profile(&mut self, profile: &CustomizationProfile) {
        let color_dot = profile
            .folder_color_target
            .as_ref()
            .map(|c| ColorDotConfig::new(c.target_r, c.target_g, c.target_b));
        self.layers.color_dot.set_config(color_dot);
        self.layers.overlay.set_config(profile.overlay.clone());
    }

    /// Exports the current SVG layer settings as a [`CustomizationProfile`].
    ///
    /// The color dot maps back to the medium-neutral `folder_color_target`
    /// intent so profiles round-trip across strategies.
    pub fn export_profile(&self) -> CustomizationProfile {
        let mut profile = CustomizationProfile::new();
        if let Some(dot) = self.layers.color_dot.config() {
            profile.folder_color_target =
                Some(FolderColorTargetConfig::new(dot.r, dot.g, dot.b));
        }
        profile.overlay = self.layers.overlay.config().cloned();
        profile
    }

    /// Clears all layer caches.
    pub fn clear_cache(&mut self) {
        self.layers.invalidate_all();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{ImageOverlayConfig, ImageSource, OverlayAnchorMode, OverlayPosition};
    use image::{Rgba, RgbaImage};

    const BASE_SVG: &str =
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#ffd970"/></svg>"##;

    fn test_base() -> SvgFolderIconBase {
        SvgFolderIconBase::new(BASE_SVG, SurfaceColor::new(255, 217, 112))
    }

    #[test]
    fn passthrough_render_returns_base_unchanged() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        assert_eq!(customizer.render_output().unwrap(), BASE_SVG);
    }

    #[test]
    fn exposes_surface_color() {
        let customizer = SvgFolderIconCustomizer::from_folder(test_base());
        assert_eq!(*customizer.surface_color(), SurfaceColor::new(255, 217, 112));
    }

    #[test]
    fn clear_cache_does_not_panic() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        customizer.clear_cache();
        assert_eq!(customizer.render_output().unwrap(), BASE_SVG);
    }

    #[test]
    fn apply_profile_adds_color_dot() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        let profile = CustomizationProfile::new()
            .with_folder_color_target(FolderColorTargetConfig::new(33, 150, 243));
        customizer.apply_profile(&profile);

        let svg = customizer.render_output().unwrap();
        assert!(svg.contains("<circle"));
        assert!(svg.contains("#2196f3"));
        // The dot is injected before the base's closing tag.
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn profile_round_trips_through_color_dot() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        let profile = CustomizationProfile::new()
            .with_folder_color_target(FolderColorTargetConfig::new(10, 20, 30));
        customizer.apply_profile(&profile);

        let exported = customizer.export_profile();
        let target = exported.folder_color_target.unwrap();
        assert_eq!((target.target_r, target.target_g, target.target_b), (10, 20, 30));
    }

    #[test]
    fn empty_profile_clears_color_dot() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        customizer.apply_profile(
            &CustomizationProfile::new()
                .with_folder_color_target(FolderColorTargetConfig::new(1, 2, 3)),
        );
        customizer.apply_profile(&CustomizationProfile::new());

        assert!(customizer.export_profile().folder_color_target.is_none());
        assert_eq!(customizer.render_output().unwrap(), BASE_SVG);
    }

    #[test]
    fn color_dot_rasterizes_into_bottom_right() {
        // De-risks the nested-`<svg>` percentage placement by rasterizing the
        // customized markup and sampling the center of the dot.
        let base = SvgFolderIconBase::new(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#cccccc"/></svg>"##,
            SurfaceColor::new(255, 217, 112),
        );
        let mut customizer = SvgFolderIconCustomizer::from_folder(base);
        customizer.apply_profile(
            &CustomizationProfile::new()
                .with_folder_color_target(FolderColorTargetConfig::new(255, 0, 0)),
        );
        let svg = customizer.render_output().unwrap();

        let img = ImageSource::svg(&svg).render_at_size(64).unwrap();
        // Center of the dot sits at ~(48, 48) for a 64px render.
        let px = img.get_pixel(48, 48).0;
        assert!(px[0] > 150, "expected red-dominant dot, got {px:?}");
        assert!(px[1] < 100 && px[2] < 100, "expected low green/blue, got {px:?}");
        assert!(px[3] > 0, "expected an opaque dot, got {px:?}");
    }

    #[test]
    fn apply_profile_adds_overlay_image() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        let profile = CustomizationProfile::new().with_overlay(ImageOverlayConfig::from_svg(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
            OverlayPosition::BottomRight,
            OverlayAnchorMode::Inset,
            0.5,
        ));
        customizer.apply_profile(&profile);

        let svg = customizer.render_output().unwrap();
        assert!(svg.contains("<image"));
        assert!(svg.contains("data:image/svg+xml;base64,"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn overlay_round_trips_through_profile() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        let profile = CustomizationProfile::new().with_overlay(ImageOverlayConfig::from_svg(
            "<svg/>",
            OverlayPosition::TopLeft,
            OverlayAnchorMode::Centered,
            0.3,
        ));
        customizer.apply_profile(&profile);

        let exported = customizer.export_profile();
        let overlay = exported.overlay.unwrap();
        assert_eq!(overlay.position, OverlayPosition::TopLeft);
        assert_eq!(overlay.anchor_mode, OverlayAnchorMode::Centered);
    }

    #[test]
    fn raster_overlay_rasterizes_into_bottom_right() {
        // De-risks the `<image>` PNG data-URI embedding + percentage placement.
        let base = SvgFolderIconBase::new(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#cccccc"/></svg>"##,
            SurfaceColor::new(255, 217, 112),
        );
        let mut customizer = SvgFolderIconCustomizer::from_folder(base);

        let blue = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 255]));
        let source = ImageSource::from_rgba_image(&blue).unwrap();
        customizer.apply_profile(&CustomizationProfile::new().with_overlay(
            ImageOverlayConfig::new(source, OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25),
        ));
        let svg = customizer.render_output().unwrap();

        let img = ImageSource::svg(&svg).render_at_size(64).unwrap();
        // The overlay occupies the bottom-right 25%; sample its center ~(56, 56).
        let px = img.get_pixel(56, 56).0;
        assert!(px[2] > 150, "expected blue-dominant overlay, got {px:?}");
        assert!(px[0] < 100 && px[1] < 100, "expected low red/green, got {px:?}");
    }

    #[test]
    fn render_preview_rasterizes_at_requested_size() {
        let mut customizer = SvgFolderIconCustomizer::from_folder(test_base());
        let preview = customizer.render_preview(64).unwrap();
        assert_eq!(preview.dimensions().width, 64);
        assert_eq!(preview.dimensions().height, 64);
    }

    #[test]
    fn render_preview_reflects_active_layers() {
        let base = SvgFolderIconBase::new(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#cccccc"/></svg>"##,
            SurfaceColor::new(255, 217, 112),
        );
        let mut customizer = SvgFolderIconCustomizer::from_folder(base);
        customizer.apply_profile(
            &CustomizationProfile::new()
                .with_folder_color_target(FolderColorTargetConfig::new(255, 0, 0)),
        );

        let preview = customizer.render_preview(64).unwrap();
        let px = preview.data.get_pixel(48, 48).0;
        assert!(px[0] > 150, "expected the color dot in the preview, got {px:?}");
    }
}
