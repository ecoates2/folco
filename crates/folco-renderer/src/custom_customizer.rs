//! Custom icon customizer — color dot + overlay layers.
//!
//! [`CustomIconCustomizer`] is a type alias for `IconCustomizer<CustomLayers>`,
//! providing color-dot badges and image overlays on user-supplied base icons.
//!
//! The solid-color and decal layers are unavailable because custom images have
//! no surface color reference to recolor against.

use crate::capabilities::{IconBaseKind, IconCapabilities};
use crate::customizer::{IconCustomizer, LayerSet};
use crate::error::RenderError;
use crate::icon::{IconBase, IconSet, IconSizeSpec};
use crate::layer::svg::composite_over;
use crate::layer::{
    CacheKey, ColorDotConfig, DependencyVersion, ImageOverlayConfig, ImageSource, Layer,
    LayerVersions, RenderContext,
};
use crate::profile::CustomizationProfile;

// ============================================================================
// CustomLayers
// ============================================================================

/// Layer set for custom icon customization — color dot and overlay.
///
/// Custom images have no surface color metadata, so the solid-color and decal
/// layers are not applicable.
#[derive(Default)]
pub struct CustomLayers {
    /// Color dot badge layer.
    pub color_dot: Layer<ColorDotConfig>,

    /// Image overlay layer.
    pub overlay: Layer<ImageOverlayConfig>,
}

impl LayerSet for CustomLayers {
    fn execute(&mut self, ctx: &mut RenderContext, key: CacheKey) -> Result<(), RenderError> {
        let versions = LayerVersions {
            solid_color: 0,
            color_dot: self.color_dot.version(),
            decal: 0,
            overlay: self.overlay.version(),
        };

        if let Some(tile) = self.color_dot.render_tile(ctx, key, &versions)? {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
        }

        if let Some(tile) = self.overlay.render_tile(ctx, key, &versions)? {
            composite_over(&mut ctx.image.data, &tile, 0, 0);
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
// CustomIconCustomizer
// ============================================================================

/// Custom icon customizer — color dot and overlay, for user-provided images.
///
/// Construct via [`CustomIconCustomizer::from_image()`] or
/// [`CustomIconCustomizer::from_icon_set()`].
///
/// # Example
///
/// ```ignore
/// use folco_renderer::{CustomIconCustomizer, ImageSource, IconSizeSpec, ImageOverlayConfig, OverlayPosition};
///
/// let source = ImageSource::svg("<svg>...</svg>");
/// let specs = vec![IconSizeSpec::square(32, 1.0), IconSizeSpec::square(256, 1.0)];
/// let mut customizer = CustomIconCustomizer::from_image(&source, &specs).unwrap();
///
/// customizer.layers.overlay.set_config(Some(
///     ImageOverlayConfig::from_svg("<svg>badge</svg>", OverlayPosition::BottomRight, OverlayAnchorMode::Inset, 0.25)
/// ));
///
/// let output = customizer.render_all();
/// ```
pub type CustomIconCustomizer = IconCustomizer<CustomLayers>;

impl CustomIconCustomizer {
    /// Creates a customizer from a user-provided image source.
    ///
    /// Each [`IconSizeSpec`] produces one base image with full-image content
    /// bounds.
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be decoded or rendered.
    pub fn from_image(source: &ImageSource, specs: &[IconSizeSpec]) -> Result<Self, RenderError> {
        let icons = IconSet::from_image_source(source, specs)?;
        Ok(IconCustomizer::new(
            IconBase::Custom(icons),
            CustomLayers::default(),
        ))
    }

    /// Creates a customizer from a pre-built icon set.
    pub fn from_icon_set(icons: IconSet) -> Self {
        IconCustomizer::new(IconBase::Custom(icons), CustomLayers::default())
    }

    /// Applies a [`CustomizationProfile`]'s settings to the layers.
    ///
    /// Intents this base can't realize are dropped — see
    /// [`capabilities`](Self::capabilities).
    pub fn apply_profile(&mut self, profile: &CustomizationProfile) {
        let profile = Self::capabilities().filter(profile);
        self.layers.color_dot.set_config(profile.color_dot);
        self.layers.overlay.set_config(profile.overlay);
    }

    /// What this customizer operates on.
    pub const fn base_kind() -> IconBaseKind {
        IconBaseKind::CustomImage
    }

    /// The layers a user-supplied image can realize.
    pub fn capabilities() -> IconCapabilities {
        Self::base_kind().capabilities()
    }

    /// Exports the current settings as a [`CustomizationProfile`].
    pub fn export_profile(&self) -> CustomizationProfile {
        CustomizationProfile {
            solid_color: None,
            color_dot: self.layers.color_dot.config().copied(),
            decal: None,
            overlay: self.layers.overlay.config().cloned(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{
        DecalConfig, OverlayAnchorMode, OverlayPosition, SolidColorConfig, SvgSource,
    };

    fn customizer() -> CustomIconCustomizer {
        CustomIconCustomizer::from_icon_set(IconSet::new())
    }

    #[test]
    fn apply_profile_keeps_draw_on_top_intents() {
        let profile = CustomizationProfile::new()
            .with_color_dot(ColorDotConfig::new(1, 2, 3))
            .with_overlay(ImageOverlayConfig::from_svg(
                "<svg></svg>",
                OverlayPosition::TopLeft,
                OverlayAnchorMode::Centered,
                0.25,
            ));

        let mut c = customizer();
        c.apply_profile(&profile);

        assert_eq!(
            c.layers.color_dot.config().unwrap(),
            &ColorDotConfig::new(1, 2, 3)
        );
        assert_eq!(
            c.layers.overlay.config().unwrap().position,
            OverlayPosition::TopLeft
        );
    }

    #[test]
    fn apply_profile_drops_surface_relative_intents() {
        let profile = CustomizationProfile::new()
            .with_solid_color(SolidColorConfig::new(76, 175, 80))
            .with_decal(DecalConfig::new("<svg></svg>", 0.5));

        let mut c = customizer();
        c.apply_profile(&profile);

        // Custom images have no surface color, so these have nowhere to land.
        let exported = c.export_profile();
        assert!(exported.solid_color.is_none());
        assert!(exported.decal.is_none());
    }

    #[test]
    fn profile_roundtrips_through_json() {
        let mut c = customizer();
        c.layers
            .color_dot
            .set_config(Some(ColorDotConfig::new(9, 8, 7)));
        c.layers
            .overlay
            .set_config(Some(ImageOverlayConfig::from_svg(
                "<svg>badge</svg>",
                OverlayPosition::BottomRight,
                OverlayAnchorMode::Inset,
                0.25,
            )));

        let json = c.export_profile().to_json().unwrap();
        let restored = CustomizationProfile::from_json(&json).unwrap();

        let mut other = customizer();
        other.apply_profile(&restored);

        assert_eq!(
            other.layers.color_dot.config().unwrap(),
            &ColorDotConfig::new(9, 8, 7)
        );
        assert_eq!(
            other.layers.overlay.config().unwrap().source,
            ImageSource::Svg(SvgSource::Raw("<svg>badge</svg>".into()))
        );
    }
}
