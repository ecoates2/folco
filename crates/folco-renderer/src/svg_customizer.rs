//! Vector customizer: the markup pipeline over a scalable SVG.
//!
//! [`SvgCustomizer`] appends layer fragments to an [`SvgCanvas`] and emits a
//! single scalable SVG string. The base can be the platform's vector folder
//! icon or a user-supplied SVG; either way only the draw-on-top layers (color
//! dot, overlay) apply — an arbitrary SVG has no surface color to shift.
//!
//! Previews rasterize the assembled markup and are cached per size until any
//! layer changes.

use std::collections::HashMap;

use crate::capabilities::IconBaseKind;
use crate::customizer::Layers;
use crate::error::RenderError;
use crate::icon::{IconImage, SvgFolderIconBase};
use crate::layer::svg::render_svg;
use crate::medium::SvgCanvas;
use crate::svg_layer::SvgStage;
use crate::ImageSource;

// ============================================================================
// PreviewCache
// ============================================================================

/// Rasterized previews by size, valid for one set of layer versions.
#[derive(Default)]
struct PreviewCache {
    versions: [u64; 2],
    by_size: HashMap<u32, IconImage>,
}

impl PreviewCache {
    /// Drops every preview if the layers have changed since they were made.
    fn sync(&mut self, versions: [u64; 2]) {
        if self.versions != versions {
            self.versions = versions;
            self.by_size.clear();
        }
    }
}

// ============================================================================
// SvgCustomizer
// ============================================================================

/// Markup pipeline over a scalable base.
///
/// Configure layers through the [`layers`](Self::layers) field, or through
/// [`Customizer`](crate::Customizer) for the capability-checked API shared
/// with the raster pipeline. Only `color_dot` and `overlay` are rendered;
/// the surface-relative layers have no vector analogue.
///
/// # Example
///
/// ```
/// use folco_renderer::{SvgFolderIconBase, SvgCustomizer};
///
/// let base = SvgFolderIconBase::new(
///     r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"></svg>"#,
/// );
/// let mut customizer = SvgCustomizer::from_folder(base);
///
/// // With no layers configured, rendering returns the base markup unchanged.
/// let svg = customizer.render_output().unwrap();
/// assert!(svg.contains("<svg"));
/// ```
pub struct SvgCustomizer {
    kind: IconBaseKind,
    base_svg: String,

    /// The layers. Only `color_dot` and `overlay` render in this medium.
    pub layers: Layers,

    preview: PreviewCache,
}

impl SvgCustomizer {
    fn new(kind: IconBaseKind, base_svg: String) -> Self {
        debug_assert!(kind.is_vector());
        Self {
            kind,
            base_svg,
            layers: Layers::default(),
            preview: PreviewCache::default(),
        }
    }

    /// Creates a customizer for the platform's scalable folder icon.
    pub fn from_folder(base: SvgFolderIconBase) -> Self {
        Self::new(IconBaseKind::VectorFolder, base.svg)
    }

    /// Creates a customizer over user-supplied SVG markup.
    ///
    /// # Errors
    ///
    /// Returns an error if the markup isn't a renderable SVG, so bad input
    /// fails here rather than on the first render.
    pub fn from_svg(svg: impl Into<String>) -> Result<Self, RenderError> {
        let svg = svg.into();
        render_svg(&svg, 1)?;
        Ok(Self::new(IconBaseKind::VectorCustomImage, svg))
    }

    // ---- Accessors ----------------------------------------------------------

    /// What this customizer operates on.
    pub fn base_kind(&self) -> IconBaseKind {
        self.kind
    }

    /// The base markup layers are drawn over.
    pub fn base_svg(&self) -> &str {
        &self.base_svg
    }

    // ---- Rendering ----------------------------------------------------------

    /// The vector pipeline, in order.
    fn stages(&self) -> [&dyn SvgStage; 2] {
        [&self.layers.color_dot, &self.layers.overlay]
    }

    /// Renders the artifact written to the system: a single scalable SVG string.
    ///
    /// With no active layers this returns the base markup unchanged.
    ///
    /// # Errors
    ///
    /// Returns a render error if a layer fails.
    pub fn render_output(&self) -> Result<String, RenderError> {
        let mut canvas = SvgCanvas::new(&self.base_svg);
        for stage in self.stages() {
            if let Some(fragment) = stage.fragment()? {
                canvas.push_overlay(fragment);
            }
        }
        Ok(canvas.into_svg())
    }

    /// Renders a rasterized preview at `size` px for on-screen display.
    ///
    /// Previews are pixels so both media can share one display path; the
    /// artifact written to the system still comes from [`render_output`](Self::render_output).
    /// The image is borrowed from the cache; clone it if it must outlive the
    /// next edit.
    ///
    /// # Errors
    ///
    /// Returns a render error if a layer fails or the SVG cannot be rasterized.
    pub fn render_preview(&mut self, size: u32) -> Result<&IconImage, RenderError> {
        self.preview.sync(self.stages().map(|s| s.version()));
        if !self.preview.by_size.contains_key(&size) {
            let svg = self.render_output()?;
            let rgba = ImageSource::svg(svg).render_at_size(size)?;
            self.preview
                .by_size
                .insert(size, IconImage::new_full_content(rgba, 1.0));
        }
        Ok(&self.preview.by_size[&size])
    }

    /// Drops every cached preview.
    pub fn clear_cache(&mut self) {
        self.preview.by_size.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{
        ColorDotConfig, ImageOverlayConfig, OverlayAnchorMode, OverlayPosition,
    };
    use crate::placement::Placement;
    use image::{Rgba, RgbaImage};

    const BASE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#ffd970"/></svg>"##;
    const GREY_100: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" fill="#cccccc"/></svg>"##;

    fn folder() -> SvgCustomizer {
        SvgCustomizer::from_folder(SvgFolderIconBase::new(BASE_SVG))
    }

    fn versions(c: &SvgCustomizer) -> [u64; 2] {
        c.stages().map(|s| s.version())
    }

    /// Catches a vector-capable layer left out of `stages()`.
    #[test]
    fn every_vector_layer_is_a_stage() {
        let baseline = versions(&folder());
        let placement = Placement::new(OverlayPosition::Center, OverlayAnchorMode::Inset, 0.5);

        let mut dot = folder();
        dot.layers.color_dot.set_config(Some(ColorDotConfig::new(1, 2, 3)));
        assert_ne!(versions(&dot), baseline, "color_dot not in stages()");

        let mut overlay = folder();
        overlay
            .layers
            .overlay
            .set_config(Some(ImageOverlayConfig::from_svg("<svg/>", placement)));
        assert_ne!(versions(&overlay), baseline, "overlay not in stages()");
    }

    #[test]
    fn base_kind_follows_the_constructor() {
        assert_eq!(folder().base_kind(), IconBaseKind::VectorFolder);
        assert_eq!(
            SvgCustomizer::from_svg(BASE_SVG).unwrap().base_kind(),
            IconBaseKind::VectorCustomImage
        );
    }

    #[test]
    fn rejects_markup_that_is_not_svg() {
        assert!(SvgCustomizer::from_svg("not an svg").is_err());
    }

    #[test]
    fn passthrough_render_returns_base_unchanged() {
        assert_eq!(folder().render_output().unwrap(), BASE_SVG);
    }

    #[test]
    fn color_dot_is_drawn_as_markup() {
        let mut c = folder();
        c.layers
            .color_dot
            .set_config(Some(ColorDotConfig::new(33, 150, 243)));

        let svg = c.render_output().unwrap();
        assert!(svg.contains("<circle"));
        assert!(svg.contains("#2196f3"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn overlay_is_embedded_as_image() {
        let mut c = folder();
        c.layers
            .overlay
            .set_config(Some(ImageOverlayConfig::from_svg(
                "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>",
                Placement::new(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.5),
            )));

        let svg = c.render_output().unwrap();
        assert!(svg.contains("<image"));
        assert!(svg.contains("data:image/svg+xml;base64,"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn color_dot_rasterizes_into_bottom_right() {
        // De-risks the nested-`<svg>` percentage placement by rasterizing the
        // customized markup and sampling the center of the dot.
        let mut c = SvgCustomizer::from_folder(SvgFolderIconBase::new(GREY_100));
        c.layers
            .color_dot
            .set_config(Some(ColorDotConfig::new(255, 0, 0)));

        let img = ImageSource::svg(c.render_output().unwrap())
            .render_at_size(64)
            .unwrap();
        // Center of the dot sits at ~(48, 48) for a 64px render.
        let px = img.get_pixel(48, 48).0;
        assert!(px[0] > 150, "expected red-dominant dot, got {px:?}");
        assert!(px[1] < 100 && px[2] < 100, "expected low green/blue, got {px:?}");
        assert!(px[3] > 0, "expected an opaque dot, got {px:?}");
    }

    #[test]
    fn raster_overlay_rasterizes_into_bottom_right() {
        // De-risks the `<image>` PNG data-URI embedding + percentage placement.
        let mut c = SvgCustomizer::from_folder(SvgFolderIconBase::new(GREY_100));
        let blue = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 255]));
        c.layers.overlay.set_config(Some(ImageOverlayConfig::new(
            ImageSource::from_rgba_image(&blue).unwrap(),
            Placement::new(OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25),
        )));

        let img = ImageSource::svg(c.render_output().unwrap())
            .render_at_size(64)
            .unwrap();
        // The overlay occupies the bottom-right 25%; sample its center ~(56, 56).
        let px = img.get_pixel(56, 56).0;
        assert!(px[2] > 150, "expected blue-dominant overlay, got {px:?}");
        assert!(px[0] < 100 && px[1] < 100, "expected low red/green, got {px:?}");
    }

    #[test]
    fn preview_rasterizes_at_requested_size_and_reflects_layers() {
        let mut c = SvgCustomizer::from_folder(SvgFolderIconBase::new(GREY_100));
        c.layers
            .color_dot
            .set_config(Some(ColorDotConfig::new(255, 0, 0)));

        let preview = c.render_preview(64).unwrap();
        assert_eq!(preview.dimensions().width, 64);
        assert_eq!(preview.dimensions().height, 64);
        let px = preview.data.get_pixel(48, 48).0;
        assert!(px[0] > 150, "expected the color dot in the preview, got {px:?}");
    }

    #[test]
    fn preview_is_cached_per_size() {
        let mut c = folder();
        c.render_preview(32).unwrap();

        assert!(c.preview.by_size.contains_key(&32));
        assert!(!c.preview.by_size.contains_key(&64));
    }

    #[test]
    fn preview_cache_invalidates_when_a_layer_changes() {
        let mut c = folder();
        let before = c.render_preview(32).unwrap().clone();

        c.layers
            .color_dot
            .set_config(Some(ColorDotConfig::new(0, 0, 255)));
        let after = c.render_preview(32).unwrap();

        assert_ne!(
            before.data.as_raw(),
            after.data.as_raw(),
            "a stale preview would hide the new color dot"
        );
    }

    #[test]
    fn clear_cache_drops_rendered_previews() {
        let mut c = folder();
        c.render_preview(32).unwrap();
        c.clear_cache();

        assert!(c.preview.by_size.is_empty());
        assert_eq!(c.render_output().unwrap(), BASE_SVG);
    }
}
