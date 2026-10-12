//! SVG custom icon customizer.
//!
//! [`SvgCustomIconCustomizer`] is the vector-medium sibling of
//! [`CustomIconCustomizer`](crate::CustomIconCustomizer): a user-supplied SVG
//! stays scalable markup, decorated with SVG layers, and is written as SVG.

use crate::capabilities::{IconBaseKind, IconCapabilities};
use crate::error::RenderError;
use crate::layer::svg::render_svg;
use crate::layer::{ColorDotConfig, DependencyVersion, ImageOverlayConfig};
use crate::medium::SvgCanvas;
use crate::profile::{CustomProfile, CustomizationProfile};
use crate::svg_folder_customizer::{SvgIconCustomizer, SvgLayerSet};
use crate::svg_layer::SvgLayer;

// ============================================================================
// SvgCustomLayers
// ============================================================================

/// Layer set for a user-supplied SVG, in composition order.
#[derive(Debug, Default)]
pub struct SvgCustomLayers {
    /// Color dot badge, drawn as a nested `<svg>`.
    pub color_dot: SvgLayer<ColorDotConfig>,

    /// Image overlay — an embedded SVG `<image>` (SVG/emoji/raster source).
    pub overlay: SvgLayer<ImageOverlayConfig>,
}

impl SvgLayerSet for SvgCustomLayers {
    fn execute(&mut self, canvas: &mut SvgCanvas<'_>) -> Result<(), RenderError> {
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
// SvgCustomIconCustomizer
// ============================================================================

/// Vector customizer over a user-supplied SVG.
pub type SvgCustomIconCustomizer = SvgIconCustomizer<SvgCustomLayers>;

impl SvgCustomIconCustomizer {
    /// Creates a customizer over user-supplied SVG markup.
    ///
    /// # Errors
    ///
    /// Returns an error if the markup isn't a renderable SVG, so bad input
    /// fails here rather than on the first render.
    pub fn from_svg(svg: impl Into<String>) -> Result<Self, RenderError> {
        let svg = svg.into();
        render_svg(&svg, 1)?;
        Ok(Self::with_layers(svg, SvgCustomLayers::default()))
    }

    /// Applies a [`CustomProfile`]'s settings to the layers.
    ///
    /// The profile is authoritative: layers it omits are cleared.
    pub fn apply_profile(&mut self, profile: CustomProfile) {
        self.layers.color_dot.set_config(profile.color_dot);
        self.layers.overlay.set_config(profile.overlay);
    }

    /// What this customizer operates on.
    pub const fn base_kind() -> IconBaseKind {
        IconBaseKind::VectorCustomImage
    }

    /// The layers a user-supplied SVG can realize.
    pub fn capabilities() -> IconCapabilities {
        Self::base_kind().capabilities()
    }

    /// Exports the currently rendering settings as a [`CustomizationProfile`].
    pub fn export_profile(&self) -> CustomizationProfile {
        CustomizationProfile {
            solid_color: None,
            color_dot: self.layers.color_dot.config().copied(),
            decal: None,
            overlay: self.layers.overlay.config().cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Render;

    const BASE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#00ff00"/></svg>"##;

    #[test]
    fn output_stays_svg_and_starts_as_the_base() {
        let mut customizer = SvgCustomIconCustomizer::from_svg(BASE_SVG).unwrap();
        assert_eq!(customizer.render_full_output().unwrap().rendered_svg, BASE_SVG);
    }

    #[test]
    fn color_dot_is_drawn_as_markup() {
        let mut customizer = SvgCustomIconCustomizer::from_svg(BASE_SVG).unwrap();
        customizer.apply_profile(CustomProfile::new().with_color_dot(ColorDotConfig::new(255, 0, 0)));

        let svg = customizer.render_full_output().unwrap().rendered_svg;
        assert!(svg.contains("<circle"));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn rejects_markup_that_is_not_svg() {
        assert!(SvgCustomIconCustomizer::from_svg("not an svg").is_err());
    }
}
