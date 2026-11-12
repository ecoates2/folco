//! One customization API over both media.
//!
//! [`Customizer`] wraps whichever pipeline the base calls for — the pixel
//! pipeline ([`RasterCustomizer`]) or the markup pipeline ([`SvgCustomizer`])
//! — and exposes a single, capability-checked surface: set a layer, apply or
//! export a profile, render. Which layers a base can realize follows from its
//! [`IconBaseKind`]; intents it can't realize are refused by
//! [`set_layer`](Customizer::set_layer) and dropped by
//! [`apply_profile`](Customizer::apply_profile), so callers configure the
//! same way regardless of what they are working on.
//!
//! Both pipelines own the same [`Layers`] struct. The vector pipeline simply
//! never runs the two surface-relative layers.

use crate::capabilities::{IconBaseKind, IconCapabilities, LayerKind, LayerRejections};
use crate::error::RenderError;
use crate::icon::{IconImage, IconSet};
use crate::layer::{
    ColorDotConfig, DecalConfig, ImageOverlayConfig, Layer, SolidColorConfig,
};
use crate::placement::DefaultPlacements;
use crate::profile::CustomizationProfile;
use crate::raster_customizer::RasterCustomizer;
use crate::svg_customizer::SvgCustomizer;

// ============================================================================
// Layers
// ============================================================================

/// Every layer a customization can carry, in pipeline order.
///
/// Shared by both media. Which of these actually render is the pipeline's
/// decision (see [`IconBaseKind::capabilities`]); the struct itself is just
/// the four slots a [`CustomizationProfile`] maps onto.
#[derive(Debug, Default)]
pub struct Layers {
    /// Recolors the whole icon against the surface color. Raster folder icons only.
    pub solid_color: Layer<SolidColorConfig>,
    /// Imprints a tinted glyph into the face. Raster folder icons only.
    pub decal: Layer<DecalConfig>,
    /// A colored badge at a placement. Every base.
    pub color_dot: Layer<ColorDotConfig>,
    /// An image at a placement. Every base.
    pub overlay: Layer<ImageOverlayConfig>,
}

impl Layers {
    /// Sets one layer from an intent. Returns true if the config changed.
    pub fn set(&mut self, intent: LayerIntent) -> bool {
        match intent {
            LayerIntent::SolidColor(c) => self.solid_color.set_config(c),
            LayerIntent::Decal(c) => self.decal.set_config(c),
            LayerIntent::ColorDot(c) => self.color_dot.set_config(c),
            LayerIntent::Overlay(c) => self.overlay.set_config(c),
        }
    }

    /// Replaces every layer from `profile`. Layers the profile omits are cleared.
    pub fn apply(&mut self, profile: CustomizationProfile) {
        self.solid_color.set_config(profile.solid_color);
        self.decal.set_config(profile.decal);
        self.color_dot.set_config(profile.color_dot);
        self.overlay.set_config(profile.overlay);
    }

    /// The current configuration of every layer, as a profile.
    pub fn export(&self) -> CustomizationProfile {
        CustomizationProfile {
            solid_color: self.solid_color.config().copied(),
            color_dot: self.color_dot.config().copied(),
            decal: self.decal.config().cloned(),
            overlay: self.overlay.config().cloned(),
        }
    }

    /// Clears every layer.
    pub fn clear(&mut self) {
        self.apply(CustomizationProfile::new());
    }
}

// ============================================================================
// LayerIntent
// ============================================================================

/// A request to set (or clear, with `None`) one layer.
#[derive(Debug, Clone)]
pub enum LayerIntent {
    SolidColor(Option<SolidColorConfig>),
    Decal(Option<DecalConfig>),
    ColorDot(Option<ColorDotConfig>),
    Overlay(Option<ImageOverlayConfig>),
}

impl LayerIntent {
    /// Which layer this intent targets.
    pub fn kind(&self) -> LayerKind {
        match self {
            Self::SolidColor(_) => LayerKind::SolidColor,
            Self::Decal(_) => LayerKind::Decal,
            Self::ColorDot(_) => LayerKind::ColorDot,
            Self::Overlay(_) => LayerKind::Overlay,
        }
    }
}

// ============================================================================
// RenderedOutput
// ============================================================================

/// The artifact written to the system, in the base's medium.
#[derive(Debug, Clone)]
pub enum RenderedOutput {
    /// A PNG set, one image per base size.
    Raster(IconSet),
    /// A single scalable SVG document.
    Vector(String),
}

// ============================================================================
// Customizer
// ============================================================================

/// A customizer over any base, in either medium.
///
/// Construct the concrete pipeline — [`RasterCustomizer`] or
/// [`SvgCustomizer`] — and wrap it with `.into()`. The medium is then an
/// implementation detail: every method here works the same way on both.
pub enum Customizer {
    /// Pixel pipeline over a PNG icon set.
    Raster(RasterCustomizer),
    /// Markup pipeline over scalable SVG.
    Vector(SvgCustomizer),
}

impl From<RasterCustomizer> for Customizer {
    fn from(c: RasterCustomizer) -> Self {
        Self::Raster(c)
    }
}

impl From<SvgCustomizer> for Customizer {
    fn from(c: SvgCustomizer) -> Self {
        Self::Vector(c)
    }
}

impl Customizer {
    // ---- What am I? ---------------------------------------------------------

    /// What this customizer operates on.
    pub fn base_kind(&self) -> IconBaseKind {
        match self {
            Self::Raster(c) => c.base_kind(),
            Self::Vector(c) => c.base_kind(),
        }
    }

    /// Which layers this base can realize.
    pub fn capabilities(&self) -> IconCapabilities {
        self.base_kind().capabilities()
    }

    /// Why each unrealizable layer is unavailable, for explaining disabled controls.
    pub fn rejections(&self) -> LayerRejections {
        self.base_kind().rejections()
    }

    /// Default overlay and color dot placement over this base.
    pub fn default_placements(&self) -> DefaultPlacements {
        self.base_kind().default_placements()
    }

    /// The base icon set, for raster bases. Vector bases have no size ladder.
    pub fn base_icons(&self) -> Option<&IconSet> {
        match self {
            Self::Raster(c) => Some(c.base_icons()),
            Self::Vector(_) => None,
        }
    }

    /// The base markup, for vector bases.
    pub fn base_svg(&self) -> Option<&str> {
        match self {
            Self::Raster(_) => None,
            Self::Vector(c) => Some(c.base_svg()),
        }
    }

    /// The layers, read-only. Prefer [`set_layer`](Self::set_layer) to mutate.
    pub fn layers(&self) -> &Layers {
        match self {
            Self::Raster(c) => &c.layers,
            Self::Vector(c) => &c.layers,
        }
    }

    fn layers_mut(&mut self) -> &mut Layers {
        match self {
            Self::Raster(c) => &mut c.layers,
            Self::Vector(c) => &mut c.layers,
        }
    }

    // ---- Configuration ------------------------------------------------------

    /// Sets one layer, if this base can realize it.
    ///
    /// Returns `false` and leaves the layer untouched when it can't (see
    /// [`IconBaseKind::rejection`]). Clearing (`None`) is always accepted.
    pub fn set_layer(&mut self, intent: LayerIntent) -> bool {
        if !self.capabilities().supports(intent.kind()) {
            return false;
        }
        self.layers_mut().set(intent);
        true
    }

    /// Applies a profile. The profile is authoritative: layers it omits are
    /// cleared, and intents this base can't realize are dropped.
    ///
    /// Dropping is silent by design — a profile saved from one icon should
    /// still apply to another. Pre-flight with
    /// [`IconBaseKind::unsupported_in`] to report instead.
    pub fn apply_profile(&mut self, profile: CustomizationProfile) {
        let realizable = self.capabilities().filter(profile);
        self.layers_mut().apply(realizable);
    }

    /// Exports what currently renders, as a profile.
    ///
    /// Only realizable layers are reported, so feeding the result back is a no-op.
    pub fn export_profile(&self) -> CustomizationProfile {
        self.capabilities().filter(self.layers().export())
    }

    /// Clears every layer, returning to the base icon.
    pub fn clear_layers(&mut self) {
        self.layers_mut().clear();
    }

    // ---- Rendering ----------------------------------------------------------

    /// Renders a rasterized preview at `size` px for on-screen display.
    ///
    /// Both media produce pixels here so a UI needs one display path. The
    /// image is borrowed from the render cache; clone it if it must outlive
    /// the next edit.
    ///
    /// # Errors
    ///
    /// Raster: [`RenderError::NoBaseIcon`] if no base image matches `size`.
    /// Either: a render error if a layer fails.
    pub fn render_preview(&mut self, size: u32) -> Result<&IconImage, RenderError> {
        match self {
            Self::Raster(c) => c.render_preview(size),
            Self::Vector(c) => c.render_preview(size),
        }
    }

    /// Renders the artifact written to the system, in this base's medium.
    ///
    /// # Errors
    ///
    /// Returns a render error if a layer fails.
    pub fn render_output(&mut self) -> Result<RenderedOutput, RenderError> {
        match self {
            Self::Raster(c) => c.render_output().map(RenderedOutput::Raster),
            Self::Vector(c) => c.render_output().map(RenderedOutput::Vector),
        }
    }

    /// Drops cached render work to free memory. Layer configuration is kept.
    pub fn clear_cache(&mut self) {
        match self {
            Self::Raster(c) => c.clear_cache(),
            Self::Vector(c) => c.clear_cache(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{FolderIconBase, SurfaceColor, SvgFolderIconBase};
    use crate::layer::{OverlayAnchorMode, OverlayPosition};
    use crate::placement::Placement;

    const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#ffd970"/></svg>"##;

    fn raster_folder() -> Customizer {
        RasterCustomizer::from_folder(FolderIconBase::new(
            IconSet::new(),
            SurfaceColor::new(255, 217, 112),
        ))
        .into()
    }

    fn vector_folder() -> Customizer {
        SvgCustomizer::from_folder(SvgFolderIconBase::new(SVG)).into()
    }

    #[test]
    fn set_layer_refuses_what_the_base_cannot_realize() {
        let mut c = vector_folder();
        let accepted = c.set_layer(LayerIntent::SolidColor(Some(SolidColorConfig::new(1, 2, 3))));

        assert!(!accepted);
        assert!(!c.layers().solid_color.is_active());
        assert!(c.export_profile().solid_color.is_none());
    }

    #[test]
    fn set_layer_accepts_what_the_base_can_realize() {
        let mut c = vector_folder();
        assert!(c.set_layer(LayerIntent::ColorDot(Some(ColorDotConfig::new(1, 2, 3)))));
        assert_eq!(
            c.export_profile().color_dot,
            Some(ColorDotConfig::new(1, 2, 3))
        );

        let mut c = raster_folder();
        assert!(c.set_layer(LayerIntent::SolidColor(Some(SolidColorConfig::new(1, 2, 3)))));
        assert!(c.layers().solid_color.is_active());
    }

    #[test]
    fn export_hides_layers_set_behind_the_api() {
        // Direct field access bypasses the capability check; export must not
        // claim the layer renders when the pipeline will never run it.
        let mut svg = SvgCustomizer::from_folder(SvgFolderIconBase::new(SVG));
        svg.layers
            .solid_color
            .set_config(Some(SolidColorConfig::new(1, 2, 3)));
        let c: Customizer = svg.into();

        assert!(c.export_profile().solid_color.is_none());
    }

    #[test]
    fn apply_profile_is_authoritative() {
        let mut c = raster_folder();
        c.apply_profile(
            CustomizationProfile::new()
                .with_solid_color(SolidColorConfig::new(1, 2, 3))
                .with_color_dot(ColorDotConfig::new(4, 5, 6)),
        );
        c.apply_profile(CustomizationProfile::new().with_color_dot(ColorDotConfig::new(7, 8, 9)));

        let exported = c.export_profile();
        assert!(exported.solid_color.is_none(), "omitted layer must be cleared");
        assert_eq!(exported.color_dot, Some(ColorDotConfig::new(7, 8, 9)));
    }

    #[test]
    fn clear_layers_returns_to_the_base() {
        let mut c = raster_folder();
        c.apply_profile(
            CustomizationProfile::new()
                .with_solid_color(SolidColorConfig::new(1, 2, 3))
                .with_overlay(ImageOverlayConfig::from_svg(
                    "<svg/>",
                    Placement::new(OverlayPosition::Center, OverlayAnchorMode::Inset, 0.5),
                )),
        );
        c.clear_layers();

        for layer in LayerKind::ALL {
            assert!(!c.export_profile().carries(layer), "{layer} still set");
        }
    }

    #[test]
    fn render_output_matches_the_medium() {
        assert!(matches!(
            raster_folder().render_output().unwrap(),
            RenderedOutput::Raster(_)
        ));
        assert!(matches!(
            vector_folder().render_output().unwrap(),
            RenderedOutput::Vector(svg) if svg == SVG
        ));
    }

    #[test]
    fn base_accessors_match_the_medium() {
        let raster = raster_folder();
        assert!(raster.base_icons().is_some());
        assert!(raster.base_svg().is_none());

        let vector = vector_folder();
        assert!(vector.base_icons().is_none());
        assert_eq!(vector.base_svg(), Some(SVG));
    }
}
